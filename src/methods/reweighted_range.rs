use ndarray::{Array2, Axis};

use super::rangevoting::fill_range_ballot;
use super::results::{Strategy, WinnerAndRunnerup};
use crate::sim::Sim;
use crate::ElectResult;

#[derive(Debug)]
pub struct RRV {
    pub strat: Strategy,
    ranks: i32,
    wtd_scores: Vec<f64>,
    ballots: Array2<i32>,
    winners: Vec<ElectResult>,
    remaining: Vec<usize>,
    k: f64,
}

/*
K = 1.0 favors large political parties. K = 0.5 favors smaller parties (more penalty).
I'm using this purely as a method of spreading out candidates across the position axes.
*/

impl RRV {
    pub fn new(sim: &Sim, ranks: i32, k: f64, strat: Strategy) -> RRV {
        RRV {
            strat,
            ranks,
            wtd_scores: vec![0.; sim.ncand],
            ballots: Array2::zeros((sim.ncit, sim.ncand)),
            winners: Vec::with_capacity(sim.ncand),
            remaining: Vec::with_capacity(sim.ncand),
            k,
        }
    }

    pub fn multi_elect(
        &mut self,
        sim: &Sim,
        _honest_rslt: Option<WinnerAndRunnerup>,
        nwinners: usize,
        _verbose: bool,
    ) -> &Vec<ElectResult> {
        self.ballots.fill(0);
        for icit in 0..sim.ncit {
            fill_range_ballot(
                &sim.scores.index_axis(Axis(0), icit),
                self.ranks,
                self.ballots
                    .index_axis_mut(Axis(0), icit)
                    .as_slice_mut()
                    .unwrap(),
            );
        }

        self.remaining.clear();
        self.remaining.extend(0..sim.ncand);
        self.winners.clear();
        while self.winners.len() < nwinners {
            self.wtd_scores.fill(0.0);
            for i in 0..sim.ncit {
                // Weight is K / (K + SUM/MAX)
                let sum = self
                    .winners
                    .iter()
                    .fold(0, |sum, j| sum + self.ballots[(i, j.cand)]);
                let wt = self.k / (self.k + (sum as f64) / ((self.ranks - 1) as f64));
                for j in self.remaining.iter() {
                    self.wtd_scores[*j] += wt * (self.ballots[(i, *j)] as f64);
                }
            }
            //println!("self.wtd_scores = {:?}", self.wtd_scores);
            // let winner = remaining.iter()
            //                       .max_by_key(|&j| self.wtd_scores[*j]).unwrap();
            // let winner_idx = remaining.iter().find(|&j| j == winner).unwrap();
            let (winner_idx, winner_score) = {
                let mut rem_iter = self.remaining.iter();
                let mut winner_idx = 0;
                let mut winner_score = self.wtd_scores[*rem_iter.next().unwrap()];
                //println!("     winner = {}, score = {}", remaining[winner_idx], winner_score);
                for (idx, j) in rem_iter.enumerate() {
                    if self.wtd_scores[*j] > winner_score {
                        winner_idx = idx + 1;
                        winner_score = self.wtd_scores[*j];
                        //println!("     New winner={}, score={}", remaining[winner_idx], winner_score);
                    }
                }
                (winner_idx, winner_score)
            };
            let winner = self.remaining.swap_remove(winner_idx);
            self.winners.push(ElectResult {
                cand: winner,
                score: winner_score,
            });
        }
        &self.winners
    }
}

#[cfg(test)]
mod tests {
    use float_eq::assert_float_eq;

    use super::*;
    use crate::sim::Sim;
    use crate::ElectResult;

    #[test]
    fn test_rrv() {
        // Using a situation described here: https://rangevoting.org/RRVr.html
        let mut sim = Sim::new(5, 100);
        let mut rrv = RRV::new(&sim, 11, 1.0, Strategy::Honest);
        for icit in 0..60 {
            // Team A
            sim.scores[(icit, 0)] = 10.; // A1
            sim.scores[(icit, 1)] = 9.; // A2
            sim.scores[(icit, 2)] = 8.; // A3
            sim.scores[(icit, 3)] = 1.; // B1
            sim.scores[(icit, 4)] = 0.; // B2
        }
        for icit in 60..100 {
            // Team B
            sim.scores[(icit, 0)] = 0.; // A1
            sim.scores[(icit, 1)] = 0.; // A2
            sim.scores[(icit, 2)] = 0.; // A3
            sim.scores[(icit, 3)] = 10.; // B1
            sim.scores[(icit, 4)] = 10.; // B2
        }

        let results = rrv.multi_elect(&sim, None, 3, true);
        assert_eq!(results.len(), 3);

        // First round, full weights, cand A1 wins with 600 pts (60 * 10 + 40 * 0)
        assert_eq!(
            results[0],
            ElectResult {
                cand: 0,
                score: 600.
            }
        );

        // Round 2
        // Team A got their favorite and are now all deweighted by half: 10 / (10 + 10)
        // Team B got nothing, no deweighting.
        // Cand B1 (idx 3) has 30 downweighted points from Team A, 400 pts from team B
        assert_eq!(
            results[1],
            ElectResult {
                cand: 3,
                score: 430.
            },
        );

        // Round 3
        let a_weight = 10. / (10. + 10. + 1.); // Winner B1 had score of 1 for A group
        let a2_score = a_weight * 9. * 60.;
        // assert_eq!(results[2], ElectResult{cand: 1, score: a2_score});
        assert_eq!(results[2].cand, 1);
        assert_float_eq!(results[2].score, a2_score, ulps <= 2); // forgive last two digits

        // Just checking the score to vote scaling. Would do this sooner but borrow checker whines.
        assert_eq!(rrv.ballots[(0, 0)], 10);
        assert_eq!(rrv.ballots[(0, 4)], 0);
    }
}
