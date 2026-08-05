//! Principal component analysis (PCA) for return matrices.
//!
//! The input is interpreted as a matrix where each inner vector is the return
//! series of one asset/variable. PCA is performed on the sample covariance matrix
//! after centreing each series to zero mean. Eigenvalues and eigenvectors are
//! computed using a symmetric eigendecomposition via `ndarray-linalg`.
//!
//! The returned eigenvectors are normalised and sorted in descending order of
//! eigenvalue. The i-th inner vector in `eigenvectors` corresponds to the i-th
//! principal component and contains the loadings for each input variable.
//!
//! Eigenvalues that are slightly negative because of numerical round-off (greater
//! than `-1e-12` times the largest eigenvalue) are clamped to zero. Larger
//! negative eigenvalues are preserved so that callers can detect numerical
//! problems.

use ndarray::Array2;
use ndarray_linalg::Eigh;
use sqt_core::{QuantError, Result};

use crate::math::{mean, validate_finite};

/// Result of a principal component analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct PcaResult {
    /// Asset/variable labels, when supplied by the caller.
    pub labels: Vec<String>,
    /// Eigenvalues sorted in descending order, length `n_components`.
    pub eigenvalues: Vec<f64>,
    /// Eigenvectors (principal component loadings) sorted by descending eigenvalue.
    /// Outer length is `n_components`; each inner vector has length equal to the
    /// number of input variables.
    pub eigenvectors: Vec<Vec<f64>>,
    /// Fraction of total variance explained by each retained component.
    pub explained_variance_ratio: Vec<f64>,
}

/// Performs principal component analysis on a returns matrix.
///
/// `returns_matrix` is a slice of asset return series. Each inner vector must have
/// the same length and contain at least two observations. `n_components` must be
/// greater than zero and no larger than the number of variables.
///
/// # Errors
///
/// Returns [`QuantError::DataQuality`] if the matrix is empty, has mismatched
/// series lengths, contains non-finite values, or if `n_components` is out of
/// range. Returns [`QuantError::Internal`] if the eigendecomposition fails.
pub fn pca(returns_matrix: &[Vec<f64>], n_components: usize) -> Result<PcaResult> {
    if returns_matrix.is_empty() {
        return Err(QuantError::DataQuality(
            "returns matrix must contain at least one series".to_string(),
        ));
    }
    if n_components == 0 || n_components > returns_matrix.len() {
        return Err(QuantError::DataQuality(format!(
            "n_components must be in [1, {}], got {n_components}",
            returns_matrix.len()
        )));
    }

    let n_vars = returns_matrix.len();
    let n_obs = returns_matrix[0].len();
    if n_obs < 2 {
        return Err(QuantError::DataQuality(
            "each return series must contain at least two observations".to_string(),
        ));
    }
    for (i, series) in returns_matrix.iter().enumerate() {
        validate_finite(series)?;
        if series.len() != n_obs {
            return Err(QuantError::DataQuality(format!(
                "series {i} has length {} but series 0 has length {n_obs}",
                series.len()
            )));
        }
    }

    // Centre each variable.
    let mut centered: Vec<Vec<f64>> = Vec::with_capacity(n_vars);
    for series in returns_matrix {
        let series_mean = mean(series);
        centered.push(series.iter().map(|&v| v - series_mean).collect());
    }

    // Build (n_vars x n_obs) data matrix and compute sample covariance.
    let data = Array2::from_shape_fn((n_vars, n_obs), |(i, t)| centered[i][t]);
    let cov = data.dot(&data.t()) / (n_obs as f64 - 1.0);

    // Symmetric eigendecomposition of the covariance matrix.
    let (eigvals, eigvecs) = cov
        .eigh(ndarray_linalg::UPLO::Lower)
        .map_err(|e| QuantError::Internal(anyhow::anyhow!("PCA eigendecomposition failed: {e}")))?;

    // Sort descending by eigenvalue.
    let mut indexed: Vec<(usize, f64)> = eigvals.iter().enumerate().map(|(i, &v)| (i, v)).collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let max_eigval = indexed.first().map(|(_, v)| *v).unwrap_or(0.0).abs();
    let eps = 1e-12 * max_eigval;
    let total_variance: f64 = eigvals
        .iter()
        .map(|&v| if v < 0.0 && v > -eps { 0.0 } else { v })
        .sum();

    let mut eigenvalues = Vec::with_capacity(n_components);
    let mut eigenvectors: Vec<Vec<f64>> = Vec::with_capacity(n_components);
    let mut explained_variance_ratio = Vec::with_capacity(n_components);

    for &(col, mut lambda) in indexed.iter().take(n_components) {
        if lambda < 0.0 && lambda > -eps {
            lambda = 0.0;
        }
        eigenvalues.push(lambda);
        eigenvectors.push((0..n_vars).map(|i| eigvecs[[i, col]]).collect());
        explained_variance_ratio.push(if total_variance == 0.0 {
            0.0
        } else {
            lambda / total_variance
        });
    }

    Ok(PcaResult {
        labels: Vec::new(),
        eigenvalues,
        eigenvectors,
        explained_variance_ratio,
    })
}
