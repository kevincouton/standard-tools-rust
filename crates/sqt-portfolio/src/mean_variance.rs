//! Mean-variance portfolio optimization.
//!
//! This module implements a fast, analytical mean-variance optimizer. Rather
//! than running a full quadratic program, it builds the sample covariance
//! matrix and uses the two-fund separation theorem:
//!
//! 1. Compute the **global minimum-variance** portfolio.
//! 2. Compute the **maximum-Sharpe** portfolio from the excess-return vector.
//! 3. If a `target_return` is supplied, blend the two funds so that the
//!    resulting portfolio has an expected return as close as possible to the
//!    target (clamped to the feasible interval). Otherwise, return the
//!    maximum-Sharpe portfolio.
//!
//! The method is numerically stabilised with a small ridge term on the diagonal
//! of the covariance matrix before inversion.

use ndarray::{Array1, Array2};
use ndarray_linalg::Inverse;
use serde::{Deserialize, Serialize};
use sqt_core::{QuantError, Result};
use std::collections::HashMap;

use crate::validation::validate_labels;

/// Small ridge added to the diagonal of the covariance matrix before inversion
/// for numerical stability.
const COV_RIDGE: f64 = 1e-8;

/// Threshold below which a denominator is treated as degenerate.
const DEGENERACY_EPS: f64 = 1e-12;

/// Result of a mean-variance optimization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeanVarianceResult {
    /// Optimized asset weights, keyed by asset label.
    pub weights: HashMap<String, f64>,
    /// Expected (per-period) return of the portfolio.
    pub expected_return: f64,
    /// Expected (per-period) volatility of the portfolio.
    pub volatility: f64,
    /// Sharpe ratio of the portfolio using the supplied risk-free rate.
    pub sharpe: f64,
}

/// Optimizes a mean-variance portfolio from a matrix of historical returns.
///
/// `returns_matrix[i]` is the return series for asset `labels[i]`. All series
/// must have the same length and contain at least two observations.
/// `risk_free_rate` is the per-period risk-free rate used for the Sharpe ratio.
/// When `target_return` is `Some(r)`, the optimizer blends the minimum-variance
/// and maximum-Sharpe portfolios to approximate `r`; when `None`, the
/// maximum-Sharpe portfolio is returned.
///
/// # Errors
///
/// Returns [`QuantError::InvalidCommand`] if the inputs are dimensionally
/// inconsistent. Returns [`QuantError::DataQuality`] if series are too short or
/// contain non-finite values. Returns [`QuantError::Internal`] if the
/// covariance matrix cannot be inverted.
pub fn optimize(
    returns_matrix: &[Vec<f64>],
    labels: &[String],
    risk_free_rate: f64,
    target_return: Option<f64>,
) -> Result<MeanVarianceResult> {
    validate(returns_matrix, labels, risk_free_rate, target_return)?;

    let n = returns_matrix.len();
    let obs = returns_matrix[0].len();

    let means = Array1::from_vec(
        returns_matrix
            .iter()
            .map(|s| s.iter().sum::<f64>() / obs as f64)
            .collect(),
    );

    // Centered data matrix (n_assets x n_observations).
    let data = Array2::from_shape_fn((n, obs), |(i, t)| returns_matrix[i][t] - means[i]);

    // Sample covariance with a small ridge for numerical stability.
    let mut cov = data.dot(&data.t()) / (obs as f64 - 1.0);
    for i in 0..n {
        cov[[i, i]] += COV_RIDGE;
    }

    let inv_cov = cov
        .inv()
        .map_err(|e| QuantError::Internal(anyhow::anyhow!("covariance inversion failed: {e}")))?;

    // Global minimum-variance portfolio: w = Sigma^{-1} 1 / (1' Sigma^{-1} 1).
    let ones = Array1::ones(n);
    let inv1 = inv_cov.dot(&ones);
    let denom_mv = inv1.sum();
    if denom_mv.abs() < DEGENERACY_EPS {
        return Err(QuantError::Internal(anyhow::anyhow!(
            "minimum-variance portfolio is degenerate"
        )));
    }
    let w_mv = &inv1 / denom_mv;

    // Maximum-Sharpe portfolio: w ∝ Sigma^{-1} (mu - rf * 1).
    let excess = &means - risk_free_rate;
    let k = inv_cov.dot(&excess);
    let sum_k = k.sum();
    let w_ms = if sum_k.abs() < DEGENERACY_EPS {
        // Excess returns are effectively zero; fall back to min-variance.
        w_mv.clone()
    } else {
        &k / sum_k
    };

    let (weights_arr, expected_return, volatility, sharpe) = if let Some(target) = target_return {
        let (ret_mv, _vol_mv, _sharpe_mv) = portfolio_metrics(&w_mv, &means, &cov, risk_free_rate);
        let (ret_ms, _vol_ms, _sharpe_ms) = portfolio_metrics(&w_ms, &means, &cov, risk_free_rate);
        let alpha = if (ret_ms - ret_mv).abs() < DEGENERACY_EPS {
            0.0
        } else {
            ((target - ret_mv) / (ret_ms - ret_mv)).clamp(0.0, 1.0)
        };
        let w = &w_ms * alpha + &w_mv * (1.0 - alpha);
        let (exp, vol, sharpe) = portfolio_metrics(&w, &means, &cov, risk_free_rate);
        (w, exp, vol, sharpe)
    } else {
        let (exp, vol, sharpe) = portfolio_metrics(&w_ms, &means, &cov, risk_free_rate);
        (w_ms, exp, vol, sharpe)
    };

    let mut weights = HashMap::with_capacity(n);
    for (i, label) in labels.iter().enumerate() {
        weights.insert(label.clone(), weights_arr[i]);
    }

    Ok(MeanVarianceResult {
        weights,
        expected_return,
        volatility,
        sharpe,
    })
}

fn portfolio_metrics(
    w: &Array1<f64>,
    means: &Array1<f64>,
    cov: &Array2<f64>,
    rf: f64,
) -> (f64, f64, f64) {
    let expected_return = w.dot(means);
    let variance = w.dot(&cov.dot(w));
    let volatility = variance.max(0.0).sqrt();
    let sharpe = if volatility > 0.0 {
        (expected_return - rf) / volatility
    } else {
        f64::NEG_INFINITY
    };
    (expected_return, volatility, sharpe)
}

fn validate(
    returns_matrix: &[Vec<f64>],
    labels: &[String],
    risk_free_rate: f64,
    target_return: Option<f64>,
) -> Result<()> {
    if !risk_free_rate.is_finite() {
        return Err(QuantError::InvalidCommand(
            "risk_free_rate must be finite".to_string(),
        ));
    }
    if let Some(target) = target_return {
        if !target.is_finite() {
            return Err(QuantError::InvalidCommand(
                "target_return must be finite".to_string(),
            ));
        }
    }
    if returns_matrix.is_empty() {
        return Err(QuantError::DataQuality(
            "returns matrix must contain at least one series".to_string(),
        ));
    }
    if labels.len() != returns_matrix.len() {
        return Err(QuantError::InvalidCommand(format!(
            "expected {} labels, got {}",
            returns_matrix.len(),
            labels.len()
        )));
    }
    validate_labels(labels)?;
    let obs = returns_matrix[0].len();
    if obs < 2 {
        return Err(QuantError::DataQuality(
            "each return series must contain at least two observations".to_string(),
        ));
    }
    for (i, series) in returns_matrix.iter().enumerate() {
        if series.len() != obs {
            return Err(QuantError::DataQuality(format!(
                "series {i} has length {} but series 0 has length {obs}",
                series.len()
            )));
        }
        if series.iter().any(|&v| !v.is_finite()) {
            return Err(QuantError::DataQuality(format!(
                "series {i} contains non-finite values"
            )));
        }
    }
    Ok(())
}
