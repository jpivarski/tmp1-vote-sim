use std::sync::Arc;

use arrow_array::builder::{Float64Builder, Int32Builder, ListBuilder, PrimitiveBuilder};
use arrow_array::types::Float64Type;
use arrow_array::{Float64Array, ListArray, RecordBatch};
use arrow_schema::{DataType, Field, SchemaBuilder};
use meansd::MeanSD;
// use std::{error::Error, sync::Arc};

use super::method::{Method, WinnerAndRunnerup};
use crate::sim::Sim;

pub struct MethodTracker {
    pub method: Box<dyn Method>,
    ntrials: usize,
    ntrials_subopt: usize,
    mean_regret: MeanSD,
    mean_subopt_regret: MeanSD,
    result_bldr: PrimitiveBuilder<Float64Type>,
}

impl MethodTracker {
    pub fn new(method: Box<dyn Method>, max_trials: usize) -> MethodTracker {
        MethodTracker {
            method,
            ntrials: 0,
            ntrials_subopt: 0,
            mean_regret: MeanSD::default(),
            mean_subopt_regret: MeanSD::default(),
            result_bldr: Float64Array::builder(max_trials),
        }
    }

    pub fn elect(
        &mut self,
        sim: &Sim,
        honest_rslt: Option<WinnerAndRunnerup>,
        verbose: bool,
    ) -> WinnerAndRunnerup {
        let result = self.method.elect(sim, honest_rslt, verbose);

        self.ntrials += 1;
        let regret = sim.regrets[result.winner.cand];
        self.mean_regret.update(regret);
        if regret > 0.0 {
            self.ntrials_subopt += 1;
            self.mean_subopt_regret.update(regret);
        }

        self.result_bldr.append_value(regret);

        result
    }

    pub fn get_field(&self) -> Field {
        Field::new(self.method.colname(), DataType::Float64, false)
    }

    pub fn get_column(&mut self) -> arrow_array::ArrayRef {
        Arc::new(self.result_bldr.finish())
    }

    pub fn report(&self) {
        let frac_suboptimal = self.ntrials_subopt as f64 / self.ntrials as f64;
        println!(
            "Method {}: Avg Regret: {}, σ: {}, Frac suboptimal winner: {}, avg subopt regret: {}",
            self.method.name(),
            self.mean_regret.mean(),
            self.mean_regret.sstdev(),
            frac_suboptimal,
            self.mean_subopt_regret.mean(),
        )
    }
}
