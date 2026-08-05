//! Black-Litterman portfolio optimization.
//!
//! This module implements the canonical Black-Litterman model. Starting from
//! market equilibrium implied returns (`Pi = delta * Sigma * w_mkt`) it blends
//! an investor's views (`P * mu = Q`) to produce posterior expected returns
//! and a posterior covariance matrix. The final weights are the mean-variance
//! optimum against the posterior estimates.
//!
//! A simplified helper, [`optimize_simplified`], accepts expert views as a
//! `HashMap<String, f64>` mapping tickers to expected returns. Each view is
//! treated as an independent, asset-specific view (`P` is a row of the identity
//! matrix and `Q` is the expected return).

use ndarray::{Array1, Array2, Axis};
use ndarray_linalg::Inverse;
use serde::{Deserialize, Serialize};
use sqt_core::{QuantError, Result};
use std::collections::HashMap;

use crate::validation::check_unique_labels;

/// Small ridge added to the diagonal of the covariance matrix before inversion
/// for numerical stability.
const COV_RIDGE: f64 = 1e-8;

/// Threshold below which a denominator is treated as degenerate.
const DEGENERACY_EPS: f64 = 1e-12;

/// Result of a Black-Litterman optimization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlackLittermanResult {
    /// Optimized asset weights, keyed by asset label.
    pub weights: HashMap<String, f64>,
    /// Posterior expected returns, keyed by asset label.
    pub expected_returns: HashMap<String, f64>,
    /// Posterior covariance matrix.
    pub covariance: Vec<Vec<f64>>,
}

/// Optimizes a portfolio using the Black-Litterman model with explicit view
/// matrices.
///
/// # Inputs
///
/// * `returns_matrix` — historical return series, one row per asset.
/// * `labels` — asset labels aligned with `returns_matrix`.
/// * `market_caps` — market capitalisation weights or absolute caps aligned
///   with `labels`.
/// * `p_matrix` — the `K x N` view matrix `P`, where each row maps a view to
///   a linear combination of asset returns.
/// * `q_vector` — the `K` vector `Q` of view expected returns.
/// * `tau` — uncertainty scaling factor for the prior covariance.
/// * `risk_aversion` — the investor's risk-aversion coefficient `delta`.
///
/// # Method
///
/// 1. Equilibrium excess returns: `Pi = delta * Sigma * w_mkt`.
/// 2. View confidence: `Omega = diag(P * (tau * Sigma) * P')`.
/// 3. Posterior returns:
///    `mu_BL = [(tau*Sigma)^{-1} + P' Omega^{-1} P]^{-1} *
///             [(tau*Sigma)^{-1} Pi + P' Omega^{-1} Q]`.
/// 4. Posterior covariance: `Sigma_BL = Sigma + [(tau*Sigma)^{-1} + P' Omega^{-1} P]^{-1}`.
/// 5. Optimal weights: `w = (delta * Sigma_BL)^{-1} * mu_BL`, normalised to sum
///    to one.
///
/// # Errors
///
/// Returns [`QuantError::InvalidCommand`] for dimension mismatches or invalid
/// parameters. Returns [`QuantError::DataQuality`] for degenerate inputs.
/// Returns [`QuantError::Internal`] if any matrix inversion fails.
pub fn optimize(
    returns_matrix: &[Vec<f64>],
    labels: &[String],
    market_caps: &[f64],
    p_matrix: &[Vec<f64>],
    q_vector: &[f64],
    tau: f64,
    risk_aversion: f64,
) -> Result<BlackLittermanResult> {
    validate(
        returns_matrix,
        labels,
        market_caps,
        p_matrix,
        q_vector,
        tau,
        risk_aversion,
    )?;

    let n = returns_matrix.len();
    let obs = returns_matrix[0].len();
    let k = p_matrix.len();

    // Sample covariance with a small ridge.
    let means: Vec<f64> = returns_matrix
        .iter()
        .map(|s| s.iter().sum::<f64>() / obs as f64)
        .collect();
    let data = Array2::from_shape_fn((n, obs), |(i, t)| returns_matrix[i][t] - means[i]);
    let mut cov = data.dot(&data.t()) / (obs as f64 - 1.0);
    for i in 0..n {
        cov[[i, i]] += COV_RIDGE;
    }

    // Market-capitalisation weights.
    let cap_sum: f64 = market_caps.iter().sum();
    let w_mkt = Array1::from_vec(market_caps.iter().map(|&c| c / cap_sum).collect());

    // Equilibrium excess returns.
    let pi = risk_aversion * cov.dot(&w_mkt);

    // Build view matrix P and vector Q.
    let p = Array2::from_shape_fn((k, n), |(i, j)| p_matrix[i][j]);
    let q = Array1::from_vec(q_vector.to_vec());

    // Omega = diag(P * (tau * Sigma) * P').
    let tau_sigma = &cov * tau;
    let mut omega_inv = Vec::with_capacity(k);
    for i in 0..k {
        let p_i = p.row(i).to_owned();
        let omega_i = p_i.dot(&tau_sigma.dot(&p_i));
        if omega_i.abs() < DEGENERACY_EPS {
            return Err(QuantError::DataQuality(format!(
                "view {i} has zero confidence (Omega[{i},{i}] = 0)"
            )));
        }
        omega_inv.push(1.0 / omega_i);
    }

    // P' Omega^{-1} P and P' Omega^{-1} Q accumulated as sums over views.
    let mut pt_op: Array2<f64> = Array2::zeros((n, n));
    let mut pt_oq: Array1<f64> = Array1::zeros(n);
    for i in 0..k {
        let p_i = p.row(i).to_owned();
        let scale = omega_inv[i];
        for a in 0..n {
            for b in 0..n {
                pt_op[[a, b]] += scale * p_i[a] * p_i[b];
            }
            pt_oq[a] += scale * q[i] * p_i[a];
        }
    }

    // (tau * Sigma)^{-1}
    let tau_sigma_inv = tau_sigma
        .inv()
        .map_err(|e| QuantError::Internal(anyhow::anyhow!("failed to invert tau*Sigma: {e}")))?;

    // M1 = (tau Sigma)^{-1} + P' Omega^{-1} P
    let m1 = &tau_sigma_inv + &pt_op;
    let m1_inv = m1
        .inv()
        .map_err(|e| QuantError::Internal(anyhow::anyhow!("failed to invert M1: {e}")))?;

    // M2 = (tau Sigma)^{-1} Pi + P' Omega^{-1} Q
    let m2 = tau_sigma_inv.dot(&pi) + pt_oq;

    let mu_bl = m1_inv.dot(&m2);
    let sigma_bl = &cov + &m1_inv;

    // Mean-variance optimal weights against posterior estimates.
    let a = &sigma_bl * risk_aversion;
    let a_inv = a.inv().map_err(|e| {
        QuantError::Internal(anyhow::anyhow!(
            "failed to invert posterior covariance: {e}"
        ))
    })?;
    let w_raw = a_inv.dot(&mu_bl);
    let w_sum: f64 = w_raw.sum();
    if w_sum.abs() < DEGENERACY_EPS {
        return Err(QuantError::Internal(anyhow::anyhow!(
            "optimised weights sum to zero"
        )));
    }
    let w = &w_raw / w_sum;

    build_result(labels, &w, &mu_bl, &sigma_bl)
}

/// Simplified Black-Litterman interface using independent expert views.
///
/// `market_caps` and `views` are keyed by ticker symbol. Every entry in
/// `views` becomes a single-row view that predicts the expected return of that
/// asset only. The view confidence matrix `Omega` is derived from the prior
/// covariance as in the full model.
pub fn optimize_simplified(
    labels: &[String],
    returns_matrix: &[Vec<f64>],
    market_caps: &HashMap<String, f64>,
    views: &HashMap<String, f64>,
    tau: f64,
    risk_aversion: f64,
) -> Result<BlackLittermanResult> {
    if labels.is_empty() {
        return Err(QuantError::DataQuality(
            "labels must not be empty".to_string(),
        ));
    }

    let mut ordered_caps = Vec::with_capacity(labels.len());
    for label in labels {
        let cap = market_caps.get(label).copied().ok_or_else(|| {
            QuantError::InvalidCommand(format!("missing market cap for asset `{label}`"))
        })?;
        ordered_caps.push(cap);
    }

    if views.is_empty() {
        return Err(QuantError::InvalidCommand(
            "at least one expert view is required".to_string(),
        ));
    }

    let mut p_rows: Vec<Vec<f64>> = Vec::with_capacity(views.len());
    let mut q: Vec<f64> = Vec::with_capacity(views.len());
    for (label, &expected_return) in views {
        let idx = labels
            .iter()
            .position(|l| l == label)
            .ok_or_else(|| QuantError::InvalidCommand(format!("unknown view asset `{label}`")))?;
        let mut row = vec![0.0; labels.len()];
        row[idx] = 1.0;
        p_rows.push(row);
        q.push(expected_return);
    }

    optimize(
        returns_matrix,
        labels,
        &ordered_caps,
        &p_rows,
        &q,
        tau,
        risk_aversion,
    )
}

fn build_result(
    labels: &[String],
    weights: &Array1<f64>,
    expected: &Array1<f64>,
    covariance: &Array2<f64>,
) -> Result<BlackLittermanResult> {
    let mut weights_map = HashMap::with_capacity(labels.len());
    let mut expected_map = HashMap::with_capacity(labels.len());
    for (i, label) in labels.iter().enumerate() {
        weights_map.insert(label.clone(), weights[i]);
        expected_map.insert(label.clone(), expected[i]);
    }

    let cov_out: Vec<Vec<f64>> = covariance
        .axis_iter(Axis(0))
        .map(|row| row.iter().copied().collect())
        .collect();

    Ok(BlackLittermanResult {
        weights: weights_map,
        expected_returns: expected_map,
        covariance: cov_out,
    })
}

#[allow(clippy::too_many_arguments)]
fn validate(
    returns_matrix: &[Vec<f64>],
    labels: &[String],
    market_caps: &[f64],
    p_matrix: &[Vec<f64>],
    q_vector: &[f64],
    tau: f64,
    risk_aversion: f64,
) -> Result<()> {
    if tau <= 0.0 || !tau.is_finite() {
        return Err(QuantError::InvalidCommand(
            "tau must be a positive finite number".to_string(),
        ));
    }
    if risk_aversion <= 0.0 || !risk_aversion.is_finite() {
        return Err(QuantError::InvalidCommand(
            "risk_aversion must be a positive finite number".to_string(),
        ));
    }
    if returns_matrix.is_empty() {
        return Err(QuantError::DataQuality(
            "returns matrix must contain at least one series".to_string(),
        ));
    }
    let n = returns_matrix.len();
    if labels.len() != n {
        return Err(QuantError::InvalidCommand(format!(
            "expected {n} labels, got {}",
            labels.len()
        )));
    }
    check_unique_labels(labels)?;
    if market_caps.len() != n {
        return Err(QuantError::InvalidCommand(format!(
            "expected {n} market caps, got {}",
            market_caps.len()
        )));
    }
    if market_caps.iter().any(|&c| c <= 0.0 || !c.is_finite()) {
        return Err(QuantError::InvalidCommand(
            "all market caps must be positive and finite".to_string(),
        ));
    }
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
    if p_matrix.is_empty() {
        return Err(QuantError::InvalidCommand(
            "P matrix must contain at least one view".to_string(),
        ));
    }
    if p_matrix.len() != q_vector.len() {
        return Err(QuantError::InvalidCommand(format!(
            "P matrix has {} rows but Q has {} elements",
            p_matrix.len(),
            q_vector.len()
        )));
    }
    for (i, row) in p_matrix.iter().enumerate() {
        if row.len() != n {
            return Err(QuantError::InvalidCommand(format!(
                "P row {i} has length {} but there are {n} assets",
                row.len()
            )));
        }
        if row.iter().all(|&v| v == 0.0) {
            return Err(QuantError::InvalidCommand(format!(
                "P row {i} is all zeros"
            )));
        }
    }
    Ok(())
}
