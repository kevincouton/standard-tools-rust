//! Robustness analysis by parameter perturbation.

use std::collections::HashMap;
use std::sync::Arc;

use sqt_core::{Ohlcv, QuantError, Result};

use crate::engine::{BacktestConfig, BacktestEngine, BacktestResult};
use crate::strategy::Strategy;

/// Statistics summarising how a metric varies across perturbations.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MetricStats {
    /// Mean total return across perturbations.
    pub mean_total_return: f64,
    /// Standard deviation of total return.
    pub std_total_return: f64,
    /// Mean maximum drawdown.
    pub mean_max_drawdown: f64,
    /// Standard deviation of maximum drawdown.
    pub std_max_drawdown: f64,
    /// Mean Sharpe ratio.
    pub mean_sharpe: f64,
    /// Standard deviation of Sharpe ratio.
    pub std_sharpe: f64,
    /// Mean win rate.
    pub mean_win_rate: f64,
    /// Standard deviation of win rate.
    pub std_win_rate: f64,
}

/// Result of a robustness analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustnessResult {
    /// Baseline result using the unperturbed parameters.
    pub base: BacktestResult,
    /// Perturbed parameter sets and their corresponding results.
    pub perturbations: Vec<(HashMap<String, String>, BacktestResult)>,
    /// Aggregate statistics across all perturbations.
    pub stats: MetricStats,
}

/// Analyser that perturbs strategy parameters and reports the stability of the
/// resulting performance metrics.
#[derive(Debug, Clone)]
pub struct RobustnessAnalyzer {
    strategy: Arc<dyn Strategy>,
    config: BacktestConfig,
}

impl RobustnessAnalyzer {
    /// Creates a new robustness analyser.
    pub fn new(strategy: Arc<dyn Strategy>, config: BacktestConfig) -> Self {
        Self { strategy, config }
    }

    /// Perturbs each numeric parameter in `base_params` by `+/- delta` and runs
    /// the strategy for every perturbation.
    ///
    /// The returned result contains the baseline run, each perturbed run, and
    /// summary statistics of total return, max drawdown, Sharpe and win rate.
    ///
    /// Non-numeric parameters listed in `deltas` are collected and reported as an
    /// error rather than silently skipped.
    pub fn analyze(
        &self,
        series: &[Ohlcv],
        base_params: &HashMap<String, String>,
        deltas: &HashMap<String, f64>,
    ) -> Result<RobustnessResult> {
        if series.is_empty() {
            return Err(QuantError::InvalidCommand(
                "robustness analysis requires a non-empty price series".to_string(),
            ));
        }

        let engine = BacktestEngine::new(self.strategy.clone(), self.config);
        let base = engine.run(series, base_params)?;

        let mut perturbations: Vec<(HashMap<String, String>, BacktestResult)> = Vec::new();
        let mut non_numeric: Vec<String> = Vec::new();

        for (key, delta) in deltas {
            if let Some(value) = base_params.get(key) {
                match parse_numeric(value) {
                    Some(NumericValue::Integer(base_value)) => {
                        let perturbed_values = [
                            ((base_value as f64) * (1.0 + delta)).round() as i64,
                            ((base_value as f64) * (1.0 - delta)).round() as i64,
                        ];
                        for pv in perturbed_values {
                            let mut params = base_params.clone();
                            params.insert(key.clone(), pv.to_string());
                            let result = engine.run(series, &params)?;
                            perturbations.push((params, result));
                        }
                    }
                    Some(NumericValue::Float(base_value)) => {
                        let perturbed_values = [
                            round_param(base_value * (1.0 + delta)),
                            round_param(base_value * (1.0 - delta)),
                        ];
                        for pv in perturbed_values {
                            let mut params = base_params.clone();
                            params.insert(key.clone(), pv);
                            let result = engine.run(series, &params)?;
                            perturbations.push((params, result));
                        }
                    }
                    None => non_numeric.push(key.clone()),
                }
            }
        }

        if !non_numeric.is_empty() {
            return Err(QuantError::InvalidCommand(format!(
                "cannot perturb non-numeric parameters: {}",
                non_numeric.join(", ")
            )));
        }

        let stats = compute_stats(&perturbations);

        Ok(RobustnessResult {
            base,
            perturbations,
            stats,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum NumericValue {
    Integer(i64),
    Float(f64),
}

fn parse_numeric(value: &str) -> Option<NumericValue> {
    // Try integer first so values like "10" are perturbed as integers.
    if let Ok(i) = value.parse::<i64>() {
        return Some(NumericValue::Integer(i));
    }
    value.parse::<f64>().ok().map(NumericValue::Float)
}

/// Rounds a perturbed floating-point parameter to a sensible number of decimal
/// places to avoid overly precise string representations.
fn round_param(value: f64) -> String {
    // Use 1e-9 as an epsilon to avoid floating-point edge cases.
    let rounded = (value / 1e-9).round() * 1e-9;
    // Trim trailing zeros for readability while preserving required precision.
    format!("{:.10}", rounded)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn compute_stats(perturbations: &[(HashMap<String, String>, BacktestResult)]) -> MetricStats {
    if perturbations.is_empty() {
        return MetricStats::default();
    }

    let total_returns: Vec<f64> = perturbations.iter().map(|(_, r)| r.total_return).collect();
    let max_drawdowns: Vec<f64> = perturbations.iter().map(|(_, r)| r.max_drawdown).collect();
    let sharpes: Vec<f64> = perturbations
        .iter()
        .map(|(_, r)| r.sharpe.unwrap_or(0.0))
        .collect();
    let win_rates: Vec<f64> = perturbations.iter().map(|(_, r)| r.win_rate).collect();

    MetricStats {
        mean_total_return: mean(&total_returns),
        std_total_return: std_dev(&total_returns),
        mean_max_drawdown: mean(&max_drawdowns),
        std_max_drawdown: std_dev(&max_drawdowns),
        mean_sharpe: mean(&sharpes),
        std_sharpe: std_dev(&sharpes),
        mean_win_rate: mean(&win_rates),
        std_win_rate: std_dev(&win_rates),
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn std_dev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let m = mean(values);
    let variance = values.iter().map(|v| (v - m).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}
