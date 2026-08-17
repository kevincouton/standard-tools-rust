//! Risk-parity portfolio construction.
//!
//! This module implements a simple **inverse-volatility** risk-parity rule. The
//! weight of each asset is proportional to the inverse of its sample volatility:
//!
//! ```text
//! w_i = (1 / sigma_i) / sum_j(1 / sigma_j)
//! ```
//!
//! Assets with lower volatility receive larger weights so that each asset
//! contributes roughly equally to the portfolio's total volatility budget. This
//! is a one-pass heuristic; more sophisticated iterative risk-budgeting can be
//! layered on top later if needed.

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

/// Computes inverse-volatility risk-parity weights from a returns matrix.
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

    let mut inv_vols = Vec::with_capacity(n);
    for series in returns_matrix {
        let mean = series.iter().sum::<f64>() / obs as f64;
        let variance = series.iter().map(|&r| (r - mean).powi(2)).sum::<f64>() / (obs as f64 - 1.0);
        let vol = variance.max(0.0).sqrt();
        inv_vols.push(if vol > DEGENERACY_EPS { 1.0 / vol } else { 0.0 });
    }

    let total = inv_vols.iter().sum::<f64>();
    if total < DEGENERACY_EPS {
        return Err(QuantError::DataQuality(
            "all assets have zero volatility; cannot compute risk-parity weights".to_string(),
        ));
    }

    let mut weights = HashMap::with_capacity(n);
    for (i, label) in labels.iter().enumerate() {
        weights.insert(label.clone(), inv_vols[i] / total);
    }

    Ok(RiskParityResult { weights })
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
