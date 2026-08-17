//! Walk-forward optimization.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, QuantError, Result};

use crate::engine::{BacktestConfig, BacktestEngine, BacktestResult};
use crate::metrics::compute_metrics;
use crate::strategy::Strategy;

/// Metric used to pick the best parameter set in-sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptimizationMetric {
    /// Maximize total return.
    #[default]
    TotalReturn,
    /// Maximize annualised Sharpe ratio.
    Sharpe,
    /// Maximize win rate.
    WinRate,
}

/// Configuration for walk-forward optimization.
#[derive(Debug, Clone)]
pub struct WalkForwardConfig {
    /// Number of bars in each in-sample training window.
    pub train_size: usize,
    /// Number of bars in each out-of-sample test window.
    pub test_size: usize,
    /// Parameter grid: each key is a parameter name and each value is a list of
    /// candidate values.
    pub param_grid: HashMap<String, Vec<String>>,
    /// Metric to optimise in-sample.
    pub metric: OptimizationMetric,
    /// Base backtest configuration.
    pub backtest: BacktestConfig,
}

/// Result of a walk-forward optimization.
#[derive(Debug, Clone, PartialEq)]
pub struct WalkForwardResult {
    /// Combined out-of-sample equity curve.
    pub equity_curve: Vec<(NaiveDate, Decimal)>,
    /// Combined out-of-sample trades.
    pub trades: Vec<crate::trade::Trade>,
    /// Combined out-of-sample total return.
    pub total_return: f64,
    /// Combined maximum drawdown.
    pub max_drawdown: f64,
    /// Combined Sharpe ratio, if computable.
    pub sharpe: Option<f64>,
    /// Total number of completed trades.
    pub number_of_trades: usize,
    /// Fraction of winning trades.
    pub win_rate: f64,
    /// Best parameters selected for each out-of-sample window.
    pub selected_params: Vec<(NaiveDate, HashMap<String, String>)>,
}

/// Maximum train/test window sizes and parameter-grid combinations to prevent
/// runaway walk-forward optimization.
pub const MAX_WALK_FORWARD_WINDOW: usize = 10_000;
pub const MAX_WALK_FORWARD_COMBINATIONS: usize = 10_000;

/// Walk-forward optimizer.
#[derive(Debug, Clone)]
pub struct WalkForwardOptimizer {
    strategy: Arc<dyn Strategy>,
    config: WalkForwardConfig,
}

impl WalkForwardOptimizer {
    /// Creates a new optimizer.
    pub fn new(strategy: Arc<dyn Strategy>, config: WalkForwardConfig) -> Self {
        Self { strategy, config }
    }

    /// Splits `series` into in-sample / out-of-sample windows, optimises
    /// parameters in-sample, and returns the combined out-of-sample result.
    pub fn run(&self, series: &[Ohlcv]) -> Result<WalkForwardResult> {
        if self.config.train_size == 0 || self.config.test_size == 0 {
            return Err(QuantError::InvalidCommand(
                "train_size and test_size must be greater than 0".to_string(),
            ));
        }
        if self.config.train_size > MAX_WALK_FORWARD_WINDOW
            || self.config.test_size > MAX_WALK_FORWARD_WINDOW
        {
            return Err(QuantError::InvalidCommand(format!(
                "train_size and test_size must be <= {MAX_WALK_FORWARD_WINDOW}"
            )));
        }

        if series.len() < self.config.train_size + self.config.test_size {
            return Err(QuantError::InvalidCommand(
                "series is too short for walk-forward configuration".to_string(),
            ));
        }

        let combinations = build_param_combinations(&self.config.param_grid);
        if combinations.is_empty() {
            return Err(QuantError::InvalidCommand(
                "walk-forward requires a non-empty parameter grid".to_string(),
            ));
        }
        if combinations.len() > MAX_WALK_FORWARD_COMBINATIONS {
            return Err(QuantError::InvalidCommand(format!(
                "parameter grid produces {} combinations; maximum is {}",
                combinations.len(),
                MAX_WALK_FORWARD_COMBINATIONS
            )));
        }

        let mut test_results: Vec<BacktestResult> = Vec::new();
        let mut selected_params: Vec<(NaiveDate, HashMap<String, String>)> = Vec::new();

        let mut start = 0;
        while start + self.config.train_size + self.config.test_size <= series.len() {
            let train_end = start + self.config.train_size;
            let test_end = train_end + self.config.test_size;

            let train = &series[start..train_end];
            let test = &series[train_end..test_end];

            let best_params = self.optimize(train, &combinations)?;
            let engine = BacktestEngine::new(self.strategy.clone(), self.config.backtest);
            let result = engine.run(test, &best_params)?;
            selected_params.push((test[0].date, best_params));
            test_results.push(result);

            start += self.config.test_size;
        }

        combine_results(test_results, selected_params, self.config.backtest)
    }

    fn optimize(
        &self,
        train: &[Ohlcv],
        combinations: &[HashMap<String, String>],
    ) -> Result<HashMap<String, String>> {
        let mut best: Option<(f64, HashMap<String, String>)> = None;

        for params in combinations {
            let engine = BacktestEngine::new(self.strategy.clone(), self.config.backtest);
            let result = engine.run(train, params)?;
            let score = match self.config.metric {
                OptimizationMetric::TotalReturn => result.total_return,
                OptimizationMetric::Sharpe => result.sharpe.unwrap_or(0.0),
                OptimizationMetric::WinRate => result.win_rate,
            };

            if best.as_ref().is_none_or(|(b, _)| score > *b) {
                best = Some((score, params.clone()));
            }
        }

        best.map(|(_, p)| p).ok_or_else(|| {
            QuantError::DataQuality("no parameter combination produced a result".to_string())
        })
    }
}

fn build_param_combinations(grid: &HashMap<String, Vec<String>>) -> Vec<HashMap<String, String>> {
    let keys: Vec<String> = grid.keys().cloned().collect();
    if keys.is_empty() {
        return vec![HashMap::new()];
    }

    let mut combinations: Vec<HashMap<String, String>> = vec![HashMap::new()];
    for key in keys {
        let values = grid.get(&key).cloned().unwrap_or_default();
        if values.is_empty() {
            continue;
        }
        let mut next = Vec::new();
        for base in &combinations {
            for value in &values {
                let mut extended = base.clone();
                extended.insert(key.clone(), value.clone());
                next.push(extended);
            }
        }
        combinations = next;
    }

    combinations
}

fn combine_results(
    results: Vec<BacktestResult>,
    selected_params: Vec<(NaiveDate, HashMap<String, String>)>,
    config: BacktestConfig,
) -> Result<WalkForwardResult> {
    if results.is_empty() {
        return Err(QuantError::InvalidCommand(
            "no out-of-sample windows were generated".to_string(),
        ));
    }

    let mut equity_curve: Vec<(NaiveDate, Decimal)> = Vec::new();
    let mut trades: Vec<crate::trade::Trade> = Vec::new();
    let mut cumulative_capital = Decimal::ZERO;

    for (window_index, result) in results.into_iter().enumerate() {
        let window_initial = result
            .equity_curve
            .first()
            .map(|(_, e)| *e)
            .unwrap_or(config.initial_capital);
        let window_final = result
            .equity_curve
            .last()
            .map(|(_, e)| *e)
            .unwrap_or(window_initial);

        // Scale the first window so it starts at initial capital; subsequent
        // windows are scaled by the cumulative capital carried forward.
        let scale = if window_index == 0 {
            if window_initial == Decimal::ZERO {
                Decimal::ZERO
            } else {
                config.initial_capital / window_initial
            }
        } else if window_initial == Decimal::ZERO {
            Decimal::ZERO
        } else {
            cumulative_capital / window_initial
        };

        for (date, equity) in result.equity_curve {
            equity_curve.push((date, equity * scale));
        }

        // Scale each trade's PnL so it contributes to the combined capital trajectory.
        for trade in result.trades {
            trades.push(crate::trade::Trade {
                pnl: trade.pnl * scale,
                ..trade
            });
        }

        cumulative_capital = window_final * scale;
    }

    // Remove duplicate dates where windows overlap (should not happen with
    // non-overlapping test windows, but defensively deduplicate).
    equity_curve.sort_by_key(|(d, _)| *d);
    equity_curve.dedup_by_key(|(d, _)| *d);

    let metrics = compute_metrics(&equity_curve, config);
    let number_of_trades = trades.len();

    let win_rate = if trades.is_empty() {
        0.0
    } else {
        let wins = trades.iter().filter(|t| t.pnl > Decimal::ZERO).count();
        wins as f64 / trades.len() as f64
    };

    Ok(WalkForwardResult {
        equity_curve,
        trades,
        total_return: metrics.total_return,
        max_drawdown: metrics.max_drawdown,
        sharpe: metrics.sharpe,
        number_of_trades,
        win_rate,
        selected_params,
    })
}
