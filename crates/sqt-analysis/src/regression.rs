//! Ordinary least squares (OLS) regression for quantitative analysis.
//!
//! This module provides a single-variable linear regression of asset returns
//! against benchmark returns. The intercept (`alpha`) is expressed in the same
//! periodic scale as the input returns (for example daily or monthly). Callers
//! that need an annualised alpha can scale it by the number of periods per year.

use sqt_core::{QuantError, Result};
use statrs::distribution::{ContinuousCDF, StudentsT};

use crate::math::{mean, validate_finite};

/// Result of a single-variable OLS regression.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearRegressionResult {
    /// Intercept of the regression, in the same periodic scale as the input returns.
    pub alpha: f64,
    /// Slope of the regression (sensitivity to the benchmark).
    pub beta: f64,
    /// Coefficient of determination.
    pub r_squared: f64,
    /// Two-sided p-value for the beta estimate under the t-distribution.
    pub p_value: f64,
    /// Regression residuals, one per observation.
    pub residuals: Vec<f64>,
}

/// Performs ordinary least squares regression of `asset_returns` on `benchmark_returns`.
///
/// Both slices must have the same non-zero length and contain at least two observations.
/// The function computes beta, alpha, R², and a two-sided p-value for beta using the
/// Student's t-distribution with `n - 2` degrees of freedom.
///
/// # Errors
///
/// Returns [`QuantError::DataQuality`] if the inputs are empty, have length one, are
/// mismatched in length, or contain non-finite values.
pub fn linear_regression(
    asset_returns: &[f64],
    benchmark_returns: &[f64],
) -> Result<LinearRegressionResult> {
    validate_finite(asset_returns)?;
    validate_finite(benchmark_returns)?;
    if asset_returns.len() != benchmark_returns.len() {
        return Err(QuantError::DataQuality(format!(
            "asset and benchmark returns must have the same length (got {} and {})",
            asset_returns.len(),
            benchmark_returns.len()
        )));
    }
    let n = asset_returns.len();
    if n < 2 {
        return Err(QuantError::DataQuality(
            "regression requires at least two observations".to_string(),
        ));
    }

    let mean_x = mean(benchmark_returns);
    let mean_y = mean(asset_returns);

    let mut ss_xx = 0.0;
    let mut ss_xy = 0.0;
    for (&x, &y) in benchmark_returns.iter().zip(asset_returns.iter()) {
        let dx = x - mean_x;
        let dy = y - mean_y;
        ss_xx += dx * dx;
        ss_xy += dx * dy;
    }

    if ss_xx == 0.0 {
        return Err(QuantError::DataQuality(
            "benchmark returns have zero variance".to_string(),
        ));
    }

    let beta = ss_xy / ss_xx;
    let alpha = mean_y - beta * mean_x;

    let mut residuals = Vec::with_capacity(n);
    let mut ss_res = 0.0;
    for (&x, &y) in benchmark_returns.iter().zip(asset_returns.iter()) {
        let fitted = alpha + beta * x;
        let resid = y - fitted;
        ss_res += resid * resid;
        residuals.push(resid);
    }

    let ss_tot = asset_returns
        .iter()
        .map(|&y| (y - mean_y).powi(2))
        .sum::<f64>();
    let r_squared = if ss_tot == 0.0 {
        0.0
    } else {
        1.0 - ss_res / ss_tot
    };

    // Standard error of beta.
    let mse = ss_res / (n as f64 - 2.0);
    let se_beta = (mse / ss_xx).sqrt();
    let t_stat = if se_beta == 0.0 {
        f64::INFINITY
    } else {
        beta / se_beta
    };

    let t_dist = StudentsT::new(0.0, 1.0, n as f64 - 2.0).map_err(|e| {
        QuantError::Internal(anyhow::anyhow!("failed to create t-distribution: {e}"))
    })?;
    let p_value = 2.0 * (1.0 - t_dist.cdf(t_stat.abs()));

    Ok(LinearRegressionResult {
        alpha,
        beta,
        r_squared,
        p_value,
        residuals,
    })
}
