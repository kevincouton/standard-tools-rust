//! Monte Carlo simulation over backtest returns.

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{thread_rng, SeedableRng};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::trade::Trade;

/// Confidence interval for a simulation metric.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfidenceInterval {
    /// Lower bound (5th percentile).
    pub lower: f64,
    /// Upper bound (95th percentile).
    pub upper: f64,
}

/// Result of a Monte Carlo simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct MonteCarloResult {
    /// Number of simulations run.
    pub simulations: usize,
    /// Confidence interval for final equity.
    pub final_equity_ci: ConfidenceInterval,
    /// Confidence interval for maximum drawdown (negative values).
    pub max_drawdown_ci: ConfidenceInterval,
}

/// Simulator that reshuffles trade returns to estimate the distribution of
/// final equity and maximum drawdown.
#[derive(Debug, Clone, Copy)]
pub struct MonteCarloSimulator {
    simulations: usize,
    seed: Option<u64>,
}

impl Default for MonteCarloSimulator {
    fn default() -> Self {
        Self {
            simulations: 1000,
            seed: None,
        }
    }
}

impl MonteCarloSimulator {
    /// Creates a simulator with the requested number of iterations and an optional
    /// seed for reproducibility.
    pub fn new(simulations: usize, seed: Option<u64>) -> Self {
        Self { simulations, seed }
    }

    /// Returns the configured number of simulations.
    pub fn simulations(&self) -> usize {
        self.simulations
    }

    /// Returns the configured seed, if any.
    pub fn seed(&self) -> Option<u64> {
        self.seed
    }

    /// Simulates by reshuffling the returns of completed trades.
    ///
    /// The returned confidence intervals use the 5th and 95th percentiles.
    pub fn from_trades(&self, trades: &[Trade], initial_capital: Decimal) -> MonteCarloResult {
        let returns: Vec<f64> = trades
            .iter()
            .map(|t| {
                let cost = t.entry_price * t.quantity;
                if cost == Decimal::ZERO {
                    0.0
                } else {
                    t.pnl.to_f64().unwrap_or(0.0) / cost.to_f64().unwrap_or(1.0)
                }
            })
            .collect();
        self.from_returns(&returns, initial_capital)
    }

    /// Simulates by reshuffling a period return series.
    pub fn from_returns(&self, returns: &[f64], initial_capital: Decimal) -> MonteCarloResult {
        if returns.is_empty() || self.simulations == 0 {
            return MonteCarloResult {
                simulations: self.simulations,
                final_equity_ci: ConfidenceInterval {
                    lower: initial_capital.to_f64().unwrap_or(0.0),
                    upper: initial_capital.to_f64().unwrap_or(0.0),
                },
                max_drawdown_ci: ConfidenceInterval {
                    lower: 0.0,
                    upper: 0.0,
                },
            };
        }

        let mut rng: Box<dyn rand::RngCore> = match self.seed {
            Some(s) => Box::new(StdRng::seed_from_u64(s)),
            None => Box::new(thread_rng()),
        };
        let mut final_equities: Vec<f64> = Vec::with_capacity(self.simulations);
        let mut max_drawdowns: Vec<f64> = Vec::with_capacity(self.simulations);

        let initial = initial_capital.to_f64().unwrap_or(0.0);
        for _ in 0..self.simulations {
            let mut shuffled = returns.to_vec();
            shuffled.shuffle(&mut rng);
            let (equity, max_dd) = simulate_path(&shuffled, initial);
            final_equities.push(equity);
            max_drawdowns.push(max_dd);
        }

        final_equities.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        max_drawdowns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        MonteCarloResult {
            simulations: self.simulations,
            final_equity_ci: ConfidenceInterval {
                lower: percentile(&final_equities, 0.05),
                upper: percentile(&final_equities, 0.95),
            },
            max_drawdown_ci: ConfidenceInterval {
                lower: percentile(&max_drawdowns, 0.05),
                upper: percentile(&max_drawdowns, 0.95),
            },
        }
    }
}

fn simulate_path(returns: &[f64], initial: f64) -> (f64, f64) {
    let mut equity = initial;
    let mut peak = equity;
    let mut max_dd = 0.0;

    for r in returns {
        equity *= 1.0 + r;
        if equity > peak {
            peak = equity;
        }
        if peak > 0.0 {
            let dd = (peak - equity) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }

    (equity, -max_dd)
}

fn percentile(values: &[f64], quantile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let index = ((quantile * (values.len().saturating_sub(1)) as f64).round() as usize)
        .min(values.len() - 1);
    values[index]
}
