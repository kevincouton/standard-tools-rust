//! Multi-factor regression for quantitative analysis.
//!
//! This module estimates an ordinary least squares regression of an asset's
//! returns on one or more factor returns. The model includes an intercept term
//! that is reported under the key `"intercept"` in the factor loadings map.
//! Coefficients are estimated by QR decomposition, which is more numerically
//! stable than the normal equations and allows rank-deficient designs to be
//! rejected gracefully.

use std::collections::HashMap;

use ndarray::{Array1, Array2};
use ndarray_linalg::{Solve, QR};
use sqt_core::{QuantError, Result};

use crate::math::{mean, validate_finite};

/// Result of a multi-factor OLS regression.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiFactorResult {
    /// Estimated coefficients, including an `"intercept"` key.
    pub factor_loadings: HashMap<String, f64>,
    /// Coefficient of determination.
    pub r_squared: f64,
    /// Idiosyncratic volatility (standard error of the residuals).
    pub idiosyncratic_volatility: f64,
}

/// Performs multi-factor OLS regression of `asset_returns` on the supplied factors.
///
/// `factors` is a map from factor name to a return series. All series, including
/// `asset_returns`, must have the same non-zero length and contain enough
/// observations to estimate the model.
///
/// # Errors
///
/// Returns [`QuantError::DataQuality`] if the inputs are empty, have mismatched
/// lengths, contain non-finite values, have insufficient observations, or if the
/// factor design matrix is rank-deficient. Returns [`QuantError::Internal`] if the
/// QR decomposition fails.
pub fn multi_factor_regression(
    asset_returns: &[f64],
    factors: &HashMap<String, Vec<f64>>,
) -> Result<MultiFactorResult> {
    validate_finite(asset_returns)?;
    if factors.is_empty() {
        return Err(QuantError::DataQuality(
            "at least one factor is required".to_string(),
        ));
    }
    if asset_returns.len() < 2 {
        return Err(QuantError::DataQuality(
            "asset return series must contain at least two observations".to_string(),
        ));
    }

    let n = asset_returns.len();
    let mut factor_names: Vec<String> = factors.keys().cloned().collect();
    factor_names.sort();

    for name in &factor_names {
        let series = &factors[name];
        validate_finite(series)?;
        if series.len() != n {
            return Err(QuantError::DataQuality(format!(
                "factor `{name}` has length {} but asset returns have length {n}",
                series.len()
            )));
        }
    }

    let n_params = factor_names.len() + 1; // factors + intercept
    if n < n_params {
        return Err(QuantError::DataQuality(format!(
            "need at least {n_params} observations to estimate {n_params} parameters, got {n}"
        )));
    }

    // Build design matrix X (n x n_params) with a leading column of ones.
    let x_data: Vec<f64> = (0..n)
        .flat_map(|t| {
            std::iter::once(1.0).chain(factor_names.iter().map(move |name| factors[name][t]))
        })
        .collect();
    let x = Array2::from_shape_vec((n, n_params), x_data)
        .map_err(|e| QuantError::Internal(anyhow::anyhow!("failed to build design matrix: {e}")))?;
    let y = Array1::from_vec(asset_returns.to_vec());

    // Solve the least-squares problem via QR decomposition: min ||X b - y||_2.
    // Rank-deficient designs produce a near-zero diagonal in R; we reject them
    // with a data-quality error rather than returning an unstable pseudo-solution.
    let (q, r) = x
        .qr()
        .map_err(|e| QuantError::Internal(anyhow::anyhow!("QR decomposition failed: {e}")))?;
    let qty = q.t().dot(&y);

    let r_diag: Vec<f64> = r.diag().iter().copied().collect();
    let max_diag = r_diag.iter().copied().fold(0.0_f64, |a, b| a.max(b.abs()));
    if max_diag > 0.0 {
        let rank_tol = 1e-12 * max_diag;
        if r_diag.iter().any(|&d| d.abs() <= rank_tol) {
            return Err(QuantError::DataQuality(
                "factor design matrix is rank-deficient or numerically singular".to_string(),
            ));
        }
    }

    let coeffs = r.solve(&qty).map_err(|_| {
        QuantError::DataQuality(
            "factor design matrix is rank-deficient or numerically singular".to_string(),
        )
    })?;

    let mut factor_loadings = HashMap::new();
    factor_loadings.insert("intercept".to_string(), coeffs[0]);
    for (idx, name) in factor_names.iter().enumerate() {
        factor_loadings.insert(name.clone(), coeffs[idx + 1]);
    }

    // Fitted values and residuals.
    let mean_y = mean(asset_returns);
    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;
    for (t, &y_t) in asset_returns.iter().enumerate() {
        let fitted = coeffs[0]
            + factor_names
                .iter()
                .enumerate()
                .map(|(idx, name)| coeffs[idx + 1] * factors[name][t])
                .sum::<f64>();
        let resid = y_t - fitted;
        ss_res += resid * resid;
        ss_tot += (y_t - mean_y).powi(2);
    }

    let r_squared = if ss_tot == 0.0 {
        0.0
    } else {
        1.0 - ss_res / ss_tot
    };
    let idiosyncratic_volatility = (ss_res / (n as f64 - n_params as f64)).sqrt();

    Ok(MultiFactorResult {
        factor_loadings,
        r_squared,
        idiosyncratic_volatility,
    })
}
