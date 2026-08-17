//! Risk-parity portfolio construction.
//!
//! This module implements an **equal-risk-contribution** risk-parity rule via a
//! damped fixed-point iteration on asset risk contributions:
//!
//! ```text
//! rc_i = w_i * (cov * w)_i
//! w_i  <- w_i + damping * (w_i * target / rc_i - w_i)
//! target = sum(rc) / n
//! ```
//!
//! The algorithm is the same one used in the C++ and Go ports and converges to
//! a portfolio where each asset contributes roughly the same amount of total
//! volatility.

use ndarray::{Array1, Array2};
use serde::{Deserialize, Serialize};
use sqt_core::{QuantError, Result};
use std::collections::HashMap;

use crate::validation::validate_labels;

/// Threshold below which a denominator is treated as degenerate.
const DEGENERACY_EPS: f64 = 1e-12;

/// Result of a risk-parity optimization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskParityResult {
    /// Risk-parity asset weights, keyed by asset label.
    pub weights: HashMap<String, f64>,
}

/// Computes equal-risk-contribution risk-parity weights from a returns matrix.
///
/// `returns_matrix[i]` is the return series for asset `labels[i]`. All series
/// must have the same length and contain at least two observations.
///
/// # Errors
///
/// Returns [`QuantError::InvalidCommand`] if the inputs are dimensionally
/// inconsistent. Returns [`QuantError::DataQuality`] if series are too short,
/// contain non-finite values, or if every asset has zero volatility.
pub fn optimize(returns_matrix: &[Vec<f64>], labels: &[String]) -> Result<RiskParityResult> {
    validate(returns_matrix, labels)?;

    let n = returns_matrix.len();
    let obs = returns_matrix[0].len();

    let means = Array1::from_vec(
        returns_matrix
            .iter()
            .map(|s| s.iter().sum::<f64>() / obs as f64)
            .collect(),
    );

    let data = Array2::from_shape_fn((n, obs), |(i, t)| returns_matrix[i][t] - means[i]);
    let cov = data.dot(&data.t()) / (obs as f64 - 1.0);

    let total_variance: f64 = (0..n).map(|i| cov[[i, i]]).sum();
    if total_variance < DEGENERACY_EPS {
        return Err(QuantError::DataQuality(
            "all assets have zero volatility; cannot compute risk-parity weights".to_string(),
        ));
    }

    let mut weights = Array1::from_elem(n, 1.0 / n as f64);

    const MAX_ITERATIONS: usize = 1000;
    const CONVERGENCE_TOL: f64 = 1e-10;
    const DAMPING: f64 = 0.5;

    for _ in 0..MAX_ITERATIONS {
        let mrc = cov.dot(&weights);
        let rc: Array1<f64> = &weights * &mrc;
        let total_rc: f64 = rc.sum();
        if total_rc < DEGENERACY_EPS {
            break;
        }
        let target = total_rc / n as f64;

        if rc.iter().all(|&r| (r - target).abs() < CONVERGENCE_TOL) {
            break;
        }

        let next: Array1<f64> = weights
            .iter()
            .zip(rc.iter())
            .map(|(&w, &r)| w + DAMPING * (w * target / r.max(DEGENERACY_EPS) - w))
            .collect();
        let next_total: f64 = next.sum();
        if next_total < DEGENERACY_EPS {
            break;
        }
        weights = &next / next_total;
    }

    let mut weights_map = HashMap::with_capacity(n);
    for (i, label) in labels.iter().enumerate() {
        weights_map.insert(label.clone(), weights[i]);
    }

    Ok(RiskParityResult {
        weights: weights_map,
    })
}

fn validate(returns_matrix: &[Vec<f64>], labels: &[String]) -> Result<()> {
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
