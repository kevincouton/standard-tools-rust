//! Risk/return metrics calculator.

use ndarray::Array1;
use sqt_core::{QuantError, Result};
use statrs::statistics::{Data, Distribution};

/// A collection of risk and return metrics.
///
/// All fields are optional because some metrics cannot be computed from
/// insufficient or degenerate input (for example, volatility with a single
/// return or Sharpe ratio with zero volatility).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MetricsResult {
    /// Sharpe ratio: excess annualised return per unit of annualised volatility.
    pub sharpe: Option<f64>,

    /// Sortino ratio: excess annualised return per unit of downside deviation.
    pub sortino: Option<f64>,

    /// Maximum peak-to-trough drawdown as a negative fraction of equity.
    pub max_drawdown: Option<f64>,

    /// Historical value at risk at the 5th percentile.
    pub var: Option<f64>,

    /// Historical conditional value at risk (expected shortfall) at the 5th percentile.
    pub cvar: Option<f64>,

    /// Beta against the provided benchmark returns.
    pub beta: Option<f64>,

    /// Annualised Jensen's alpha against the provided benchmark returns.
    pub alpha: Option<f64>,

    /// Annualised geometric mean return, scaled by `periods_per_year`.
    pub annualized_return: Option<f64>,

    /// Annualised standard deviation of returns, scaled by `periods_per_year`.
    pub volatility: Option<f64>,

    /// Compounded total return over the observed period.
    pub total_return: Option<f64>,
}

/// Calculator for risk and return metrics.
///
/// The primary API is [`MetricsCalculator::from_returns`], which accepts a
/// slice of period returns, an annualised risk-free rate, an optional set of
/// benchmark returns, and the number of periods per year used to annualise
/// returns and volatility.
pub struct MetricsCalculator;

impl MetricsCalculator {
    /// Compute risk/return metrics from a return series.
    ///
    /// # Arguments
    ///
    /// * `returns` — period returns, e.g. daily returns.
    /// * `risk_free_rate` — annualised risk-free rate as a decimal (e.g. 0.02).
    /// * `benchmark_returns` — optional benchmark returns of the same length as
    ///   `returns`. If supplied, beta and alpha are computed.
    /// * `periods_per_year` — number of periods in one year, used to annualise
    ///   returns and volatility (e.g. 252 for daily trading days, 12 for monthly).
    ///
    /// # Errors
    ///
    /// Returns [`QuantError::InvalidCommand`] if `benchmark_returns` is
    /// supplied with a length different from `returns`.
    pub fn from_returns(
        returns: &[f64],
        risk_free_rate: f64,
        benchmark_returns: Option<&[f64]>,
        periods_per_year: u32,
    ) -> Result<MetricsResult> {
        if returns.is_empty() {
            return Ok(MetricsResult::default());
        }

        if periods_per_year == 0 {
            return Err(QuantError::InvalidCommand(
                "periods_per_year must be greater than zero".to_string(),
            ));
        }

        if let Some(bench) = benchmark_returns {
            if bench.len() != returns.len() {
                return Err(QuantError::InvalidCommand(
                    "benchmark returns length must match returns length".to_string(),
                ));
            }
        }

        let total_return = Some(compute_total_return(returns));
        let annualized_return =
            total_return.map(|tr| compute_annualized_return(tr, returns.len(), periods_per_year));
        let volatility = Some(compute_annualized_volatility(returns, periods_per_year));

        let sharpe = match (annualized_return, volatility) {
            (Some(ann), Some(vol)) if vol > 0.0 => Some((ann - risk_free_rate) / vol),
            _ => None,
        };

        let sortino = compute_sortino_ratio(returns, risk_free_rate, periods_per_year);
        let max_drawdown = Some(compute_max_drawdown(returns));
        let var = historical_var(returns, 0.05);
        let cvar = historical_cvar(returns, 0.05);

        let (beta, alpha) = if let Some(bench) = benchmark_returns {
            let b = compute_beta(returns, bench);
            let bench_total = compute_total_return(bench);
            let bench_annual =
                compute_annualized_return(bench_total, bench.len(), periods_per_year);
            let a = compute_alpha(
                annualized_return.unwrap_or(0.0),
                risk_free_rate,
                b,
                bench_annual,
            );
            (Some(b), Some(a))
        } else {
            (None, None)
        };

        Ok(MetricsResult {
            sharpe,
            sortino,
            max_drawdown,
            var,
            cvar,
            beta,
            alpha,
            annualized_return,
            volatility,
            total_return,
        })
    }
}

fn compute_total_return(returns: &[f64]) -> f64 {
    returns.iter().fold(1.0, |acc, r| acc * (1.0 + r)) - 1.0
}

fn compute_annualized_return(total_return: f64, n: usize, periods_per_year: u32) -> f64 {
    (1.0 + total_return).powf(periods_per_year as f64 / n as f64) - 1.0
}

fn compute_annualized_volatility(returns: &[f64], periods_per_year: u32) -> f64 {
    if returns.len() < 2 {
        return 0.0;
    }
    let data = Data::new(returns.to_vec());
    let mean = data.mean().unwrap_or(0.0);
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
    variance.sqrt() * (periods_per_year as f64).sqrt()
}

fn compute_sortino_ratio(
    returns: &[f64],
    risk_free_rate: f64,
    periods_per_year: u32,
) -> Option<f64> {
    if returns.is_empty() {
        return None;
    }
    let total = compute_total_return(returns);
    let annualized = compute_annualized_return(total, returns.len(), periods_per_year);
    let periodic_rf = risk_free_rate / periods_per_year as f64;

    let downside_sum: f64 = returns
        .iter()
        .map(|r| {
            let diff = r - periodic_rf;
            if diff < 0.0 {
                diff * diff
            } else {
                0.0
            }
        })
        .sum();
    let downside_deviation =
        (downside_sum / returns.len() as f64).sqrt() * (periods_per_year as f64).sqrt();

    if downside_deviation > 0.0 {
        Some((annualized - risk_free_rate) / downside_deviation)
    } else {
        None
    }
}

fn compute_max_drawdown(returns: &[f64]) -> f64 {
    let mut peak = 1.0;
    let mut max_dd = 0.0;

    for r in returns {
        let value = peak * (1.0 + r);
        if value > peak {
            peak = value;
        }
        let drawdown = (peak - value) / peak;
        if drawdown > max_dd {
            max_dd = drawdown;
        }
    }

    -max_dd
}

/// Historical value at risk at the requested quantile.
///
/// Uses the nearest-rank method: returns are sorted and the element at
/// `floor(quantile * (len - 1))` is selected. For the default 5% quantile this
/// returns the worst return that is still within the best 95% of observations.
fn historical_var(returns: &[f64], quantile: f64) -> Option<f64> {
    if returns.is_empty() {
        return None;
    }
    let mut sorted = returns.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let index = ((quantile * (sorted.len().saturating_sub(1)) as f64).round() as usize)
        .min(sorted.len() - 1);
    Some(sorted[index])
}

fn historical_cvar(returns: &[f64], quantile: f64) -> Option<f64> {
    let var = historical_var(returns, quantile)?;
    let tail: Vec<_> = returns.iter().copied().filter(|r| *r <= var).collect();
    if tail.is_empty() {
        return None;
    }
    Some(tail.iter().sum::<f64>() / tail.len() as f64)
}

fn compute_beta(returns: &[f64], benchmark: &[f64]) -> f64 {
    let r = Array1::from(returns.to_vec());
    let b = Array1::from(benchmark.to_vec());

    let r_mean = r.iter().sum::<f64>() / r.len() as f64;
    let b_mean = b.iter().sum::<f64>() / b.len() as f64;

    let covariance = r
        .iter()
        .zip(b.iter())
        .map(|(ri, bi)| (ri - r_mean) * (bi - b_mean))
        .sum::<f64>()
        / r.len() as f64;

    let b_variance = b.iter().map(|bi| (bi - b_mean).powi(2)).sum::<f64>() / b.len() as f64;

    if b_variance == 0.0 {
        0.0
    } else {
        covariance / b_variance
    }
}

fn compute_alpha(
    annualized_return: f64,
    risk_free_rate: f64,
    beta: f64,
    benchmark_annualized_return: f64,
) -> f64 {
    annualized_return - (risk_free_rate + beta * (benchmark_annualized_return - risk_free_rate))
}
