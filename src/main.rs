extern crate rand;
#[macro_use]
extern crate lazy_static;
//#[macro_use(s)]
extern crate ndarray;
use ndarray::{Array2, Axis};

mod consideration;
mod methods;

use consideration::*;
use methods::*;

const CITIZENS: usize = 10001;
const CANDIDATES_PRE: usize = 30;
const CANDIDATES: usize = 5;

fn main() {
    let mut rng = rand::thread_rng();

    let axes: [&Consideration; 3] = [
        &Likability{
            stretch_factor: 0.5,
        },
        &Issue{
            sigma: 2.0,
        },
        &Issue{
            sigma: 0.5,
        },
    ];
    let mut net_scores = unsafe { Array2::uninitialized((CITIZENS, CANDIDATES)) };
    {
        let mut net_scores_pre: Array2<f64> = Array2::zeros((CITIZENS, CANDIDATES_PRE));
        let mut scores = unsafe { Array2::uninitialized((CITIZENS, CANDIDATES_PRE)) };
        for ax in axes.iter() {
            ax.gen_scores(&mut scores, &mut rng);
            //println!("scores:\n{:?}", scores);
            for (sc, nsc) in scores.iter().zip(net_scores_pre.iter_mut()) {
                *nsc += *sc;
            }
        }
        let final_candidates = rrv(&net_scores_pre, 10, CANDIDATES);
        println!("Pre-election winners: {:?}", final_candidates);
        for (i, sv) in net_scores_pre.axis_iter(Axis(0)).enumerate() {
            for (jidx, j) in final_candidates.iter().enumerate() {
                net_scores[(i, jidx)] = sv[*j];
            }
        }
    }

    let regs = regrets(&net_scores);

    //println!("Net scores:\n{:?}", net_scores);
    let plh_result = elect_plurality_honest(&net_scores);
    println!("Plurality, honest:");
    print_score(&plh_result, &regs);

    let pls_result = elect_plurality_strategic(&net_scores, 1.0, &plh_result);
    println!("Plurality, strategic:");
    print_score(&pls_result, &regs);

    let r10h_result = elect_range_honest(&net_scores, 10);
    println!("Range<10>, honest:");
    print_score(&r10h_result, &regs);

    let r10s_result = elect_range_strategic(&net_scores, 10, 1.0, &r10h_result);
    println!("Range<10>, strategic:");
    print_score(&r10s_result, &regs);
}

fn print_score(result: &(Result, Result), regs: &Vec<f64>) {
    let r1 = regs[result.0.cand];
    let vic_margin = (result.0.score - result.1.score) / result.0.score;
    println!("  cand {} won, {} is runup, {:.2}% margin, {:.4} regret",
        result.0.cand, result.1.cand, vic_margin * 100.0, r1
    )
}
