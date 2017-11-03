extern crate rand;
#[macro_use]
extern crate lazy_static;
//#[macro_use(s)]
extern crate ndarray;
extern crate docopt;
extern crate csv;

use ndarray::{Array2, Axis};
use docopt::Docopt;
use std::error::Error;
use std::process;
use std::fs::File;
use std::io::prelude::*;

// Local "libraries"
mod consideration;
mod methods;
use consideration::*;
use methods::*;

const USAGE: &'static str = "
Simulated voting

Usage: voting [options]
       voting (--help | --version)

Options:
    -h, --help                   Show this message
    --version                    Show the version of voting
    -v NCIT --voters=NCIT        Number of voters [default: 11]
    -c NCAND --candidates=NCAND  Number of candidates [default: 5]
    -p NPCAND --primcand=NPCAND  Number of preelection candidates [default: 7]
    --likefctr=FACT              Likeability factor [default: 0.0]
    -o CSVFILE --out CSVFILE     CSV output file [default: out.csv]
    -t TRIALS                    Run TRIALS elections [default: 1]
";

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<Error>> {
    let args = Docopt::new(USAGE)
        .and_then(|dopt| dopt.parse())
        .unwrap_or_else(|e| e.exit());
    println!("{:?}", args);

    let ncit: usize = args.get_str("--voters").parse().unwrap();
    let npcand: usize = args.get_str("--primcand").parse().unwrap();
    let ncand: usize = args.get_str("--candidates").parse().unwrap();
    let lfact: f64 = args.get_str("--likefctr").parse().unwrap();
    let csvfile = args.get_str("--out");
    let trials: usize = args.get_str("-t").parse().unwrap();

    println!("Creating CSV file {}", csvfile);
    let mut ofile = File::create(csvfile)?;
    {
        let mut wtr = csv::Writer::from_writer(&ofile);
        wtr.write_record(&["Citizens", "PrimCands", "Candidates", "LikeFact", "Issue1Sigma", "Issue2Sigma"])?;
        wtr.serialize((ncit, npcand, ncand, lfact, 2.0, 0.5))?;
    }
    ofile.write_all(b"\n")?;

    let axes: [&Consideration; 2] = [
        &Likability{
            stretch_factor: lfact,
        },
        // &Issue{
        //     sigma: 2.0,
        //     halfcsep: 0.0,
        //     halfvsep: 0.0,
        // },
        // &Issue{
        //     sigma: 0.5,
        //     halfcsep: 0.0,
        //     halfvsep: 0.0,
        // },
        &MDIssue{
            issues: vec![
                Issue{
                    sigma: 1.0,
                    halfcsep: 1.0,
                    halfvsep: 1.0,
                },
                Issue{
                    sigma: 0.5,
                    halfcsep: 0.0,
                    halfvsep: 0.0,
                },
            ],
        }
    ];

    let mut net_scores = unsafe { Array2::uninitialized((ncit, ncand)) };
    let mut rng = rand::thread_rng();
    let mut wtr = csv::Writer::from_writer(&ofile);
    wtr.write_record(&["SPlMargin", "PlRegret", "SPlRegret", "R10Regret", "SR10Regret",
        "AppRegret", "SAppRegret", "IRVRegret"])?;

    for itrial in 0..trials {
        println!("Starting trial {}", itrial);
        {
            let mut net_scores_pre: Array2<f64> = Array2::zeros((ncit, npcand));
            let mut scores = unsafe { Array2::uninitialized((ncit, npcand)) };
            for ax in axes.iter() {
                ax.gen_scores(&mut scores, &mut rng, itrial==0);
                if itrial == 0 && ncit < 20 {
                    println!("scores for {:?}:\n{:?}", ax, scores);
                }
                for (sc, nsc) in scores.iter().zip(net_scores_pre.iter_mut()) {
                    *nsc += *sc;
                }
            }
            let regs_pre = regrets(&net_scores_pre);
            let mut final_candidates = rrv(&net_scores_pre, 10, ncand);
            if itrial == 0 {
                println!("Pre-election winners: {:?}", final_candidates);
            }
            final_candidates.sort_by(|&a, &b| regs_pre[a].partial_cmp(&regs_pre[b]).unwrap());
            for (i, sv) in net_scores_pre.axis_iter(Axis(0)).enumerate() {
                for (jidx, j) in final_candidates.iter().enumerate() {
                    net_scores[(i, jidx)] = sv[*j];
                }
            }
        }
        if itrial == 0 && ncit < 20 {
            println!("net_scores:\n{:?}", net_scores);
        }

        let regs = regrets(&net_scores);
        // if itrial == 0 {
        println!("Regrets: {:?}", regs);
        // }

        let cov_mat = get_cov_matrix(&net_scores);
        println!("Covariance matrix for candidates:");
        for ix in 0..ncand {
            print!(" [{}] ", ix);
            for iy in 0..(ix+1) {
                print!(" {:11.6}", cov_mat[(ix, iy)]);
            }
            println!("");
        }

        //println!("Net scores:\n{:?}", net_scores);
        let plh_result = elect_plurality_honest(&net_scores, itrial==0);
        if itrial == 0 {
            println!("Plurality, honest:");
            print_score(&plh_result, &regs);
        }

        let pls_result = elect_plurality_strategic(&net_scores, 1.0, &plh_result, itrial==0);
        if itrial == 0 {
            println!("Plurality, strategic:");
            print_score(&pls_result, &regs);
        }

        let r10h_result = elect_range_honest(&net_scores, 10, itrial==0);
        if itrial == 0 {
            println!("Range<10>, honest:");
            print_score(&r10h_result, &regs);
        }

        let r10s_result = elect_range_strategic(&net_scores, 10, 1.0, &r10h_result, itrial==0);
        if itrial == 0 {
            println!("Range<10>, strategic:");
            print_score(&r10s_result, &regs);
        }

        let r2h_result = elect_range_honest(&net_scores, 2, itrial==0);
        if itrial == 0 {
            println!("Approval, honest:");
            print_score(&r2h_result, &regs);
        }

        let r2s_result = elect_range_strategic(&net_scores, 2, 1.0, &r2h_result, itrial==0);
        if itrial == 0 {
            println!("Approval, strategic:");
            print_score(&r2s_result, &regs);
        }

        let ranked_ballots = get_ranked_ballots(&net_scores);
        if itrial == 0 && ncit < 20 {
            println!("Ranked ballots:\n{:?}", ranked_ballots);
        }
        let irv_result = elect_irv_honest(&ranked_ballots, itrial==0);
        let mut irv_winner = ncand;
        if let Some(winner) = irv_result {
            if itrial == 0 {
                println!("IRV winner is {}, {:.4} regret", winner, regs[winner]);
            }
            irv_winner = winner;
        } else {
            println!("No IRV winner -- Huh??");
        }

        let spl_margin = (pls_result.0.score - pls_result.1.score) / pls_result.0.score;
        wtr.serialize((spl_margin,
            regs[plh_result.0.cand],
            regs[pls_result.0.cand],
            regs[r10h_result.0.cand],
            regs[r10s_result.0.cand],
            regs[r2h_result.0.cand],
            regs[r2s_result.0.cand],
            regs[irv_winner],
        ))?;
    }
    Ok(())
}

fn print_score(result: &(ElectResult, ElectResult), regs: &Vec<f64>) {
    let r1 = regs[result.0.cand];
    let vic_margin = (result.0.score - result.1.score) / result.0.score;
    println!("  cand {} won, {} is runup, {:.2}% margin, {:.4} regret",
        result.0.cand, result.1.cand, vic_margin * 100.0, r1
    )
}
