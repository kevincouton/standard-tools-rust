//! Pearson correlation matrix computation.
//!
//! This module builds a symmetric correlation matrix from a map of return series.
//! Labels are sorted alphabetically to give a stable ordering.

use std::collections::HashMap;

use sqt_core::{QuantError, Result};

use crate::math::{mean, validate_finite};

/// A symmetric Pearson correlation matrix with asset labels.
#[derive(Debug, Clone, PartialEq)]
pub struct CorrelationMatrix {
    /// Ordered asset labels.
    pub labels: Vec<String>,
    /// Symmetric correlation matrix; entries are in `[-1, 1]`.
    pub matrix: Vec<Vec<f64>>,
}

/// Computes a Pearson correlation matrix from a map of return series.
///
/// All series must have the same non-zero length and contain at least two
/// observations. The returned labels are sorted alphabetically.
///
/// # Errors
///
/// Returns [`QuantError::DataQuality`] if the map is empty, lengths differ, a
/// series has zero variance, or any input contains non-finite values.
pub fn correlation(returns_map: &HashMap<String, Vec<f64>>) -> Result<CorrelationMatrix> {
    if returns_map.is_empty() {
        return Err(QuantError::DataQuality(
            "returns map must contain at least one series".to_string(),
        ));
    }

    let mut labels: Vec<String> = returns_map.keys().cloned().collect();
    labels.sort();

    let n = returns_map[&labels[0]].len();
    if n < 2 {
        return Err(QuantError::DataQuality(
            "each return series must contain at least two observations".to_string(),
        ));
    }
    for label in &labels {
        let series = &returns_map[label];
        validate_finite(series)?;
        if series.len() != n {
            return Err(QuantError::DataQuality(format!(
                "series `{label}` has length {} but expected {n}",
                series.len()
            )));
        }
    }

    let means: Vec<f64> = labels.iter().map(|l| mean(&returns_map[l])).collect();

    let mut cov = vec![vec![0.0; labels.len()]; labels.len()];
    let mut variances = vec![0.0; labels.len()];

    for (i, label_i) in labels.iter().enumerate() {
        for (j, label_j) in labels.iter().enumerate().skip(i) {
            let mut acc = 0.0;
            for (x_i, x_j) in returns_map[label_i].iter().zip(&returns_map[label_j]) {
                acc += (x_i - means[i]) * (x_j - means[j]);
            }
            let value = acc / (n as f64 - 1.0);
            cov[i][j] = value;
            if i == j {
                variances[i] = value;
            } else {
                cov[j][i] = value;
            }
        }
    }

    let mut matrix = vec![vec![0.0; labels.len()]; labels.len()];
    for (i, row) in matrix.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            let denom = (variances[i] * variances[j]).sqrt();
            if denom == 0.0 {
                return Err(QuantError::DataQuality(format!(
                    "series `{}` and/or `{}` have zero variance",
                    labels[i], labels[j]
                )));
            }
            *cell = cov[i][j] / denom;
        }
    }

    Ok(CorrelationMatrix { labels, matrix })
}
