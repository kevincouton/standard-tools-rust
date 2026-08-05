//! Cointegration analysis using the Engle-Granger two-step procedure.
//!
//! The implementation follows the classic Engle-Granger approach:
//!
//! 1. Regress the price series `a` on `b` using ordinary least squares. The slope
//!    is the estimated hedge ratio.
//! 2. Examine the resulting residuals. Because a full Augmented Dickey-Fuller (ADF)
//!    test requires dedicated time-series dependencies, this crate uses R/S analysis
//!    applied directly to the residual *levels* as a practical stationarity proxy.
//!    Unlike the price-based Hurst estimator, this does not shift residuals or apply
//!    a log-return transform, so the estimate is translation-invariant.
//!
//!    The p-value is a heuristic: under the null hypothesis of a random walk the
//!    expected Hurst exponent is `0.5`. Values materially below `0.5` indicate
//!    mean-reversion and therefore a lower p-value. The mapping uses a normal
//!    approximation with standard error `1 / sqrt(n)`; it is a rule of thumb rather
//!    than an exact ADF p-value.
//!
//! The half-life of mean reversion is estimated by regressing the residual at time
//! `t` on the residual at time `t - 1`. If the autoregressive coefficient `phi` lies
//! in `(0, 1)`, the half-life is `-ln(2) / ln(phi)`.

use sqt_core::{QuantError, Result};
use statrs::distribution::{ContinuousCDF, Normal};

use crate::hurst::hurst_exponent_rs_levels;
use crate::math::{mean, validate_finite};

/// Result of a cointegration test between two price series.
#[derive(Debug, Clone, PartialEq)]
pub struct CointegrationResult {
    /// OLS slope of `a` regressed on `b`; the number of units of `b` to hold for each
    /// unit of `a` in a mean-reverting spread.
    pub hedge_ratio: f64,
    /// Estimated half-life of mean reversion, in the same periodicity as the input.
    pub half_life: f64,
    /// Two-sided heuristic p-value for the R/S stationarity proxy of the residuals.
    pub p_value: f64,
    /// Final normalised z-score of the residual spread.
    pub z_score: f64,
}

/// Tests for cointegration between two price series using the Engle-Granger procedure.
///
/// The input slices must have the same non-zero length and contain at least ten
/// observations. The returned `hedge_ratio` is the beta from regressing `a_closes`
/// on `b_closes`; `z_score` is the last residual standardised by its sample standard
/// deviation.
///
/// # Errors
///
/// Returns [`QuantError::DataQuality`] if the inputs are mismatched, too short,
/// contain non-finite values, or have insufficient variation.
pub fn cointegration(a_closes: &[f64], b_closes: &[f64]) -> Result<CointegrationResult> {
    validate_finite(a_closes)?;
    validate_finite(b_closes)?;
    if a_closes.len() != b_closes.len() {
        return Err(QuantError::DataQuality(format!(
            "price series must have the same length (got {} and {})",
            a_closes.len(),
            b_closes.len()
        )));
    }
    let n = a_closes.len();
    if n < 10 {
        return Err(QuantError::DataQuality(
            "cointegration test requires at least ten observations".to_string(),
        ));
    }

    let mean_a = mean(a_closes);
    let mean_b = mean(b_closes);

    let mut ss_bb = 0.0;
    let mut ss_ab = 0.0;
    for (&a, &b) in a_closes.iter().zip(b_closes.iter()) {
        let db = b - mean_b;
        ss_bb += db * db;
        ss_ab += (a - mean_a) * db;
    }

    if ss_bb == 0.0 {
        return Err(QuantError::DataQuality(
            "benchmark series has zero variance".to_string(),
        ));
    }

    let hedge_ratio = ss_ab / ss_bb;
    let alpha = mean_a - hedge_ratio * mean_b;

    let residuals: Vec<f64> = a_closes
        .iter()
        .zip(b_closes.iter())
        .map(|(&a, &b)| a - (alpha + hedge_ratio * b))
        .collect();

    // Half-life via AR(1) coefficient of the residuals.
    let phi = ar1_coefficient(&residuals);
    let half_life = if phi > 0.0 && phi < 1.0 {
        -2.0_f64.ln() / phi.ln()
    } else {
        f64::INFINITY
    };

    // Stationarity proxy: R/S analysis applied directly to residual levels.
    // This avoids the log-return transform used by the price-based Hurst estimator,
    // which is not translation-invariant and therefore inappropriate for residuals
    // that can be negative.
    let hurst = hurst_exponent_rs_levels(&residuals);
    let p_value = hurst_p_value(hurst, residuals.len());

    let z_score = standard_score(*residuals.last().unwrap(), &residuals);

    Ok(CointegrationResult {
        hedge_ratio,
        half_life,
        p_value,
        z_score,
    })
}

fn ar1_coefficient(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mut num = 0.0;
    let mut den = 0.0;
    for window in values.windows(2) {
        num += window[1] * window[0];
        den += window[0] * window[0];
    }
    if den == 0.0 {
        0.0
    } else {
        num / den
    }
}

fn standard_score(value: f64, values: &[f64]) -> f64 {
    let m = mean(values);
    let variance = values.iter().map(|&v| (v - m).powi(2)).sum::<f64>() / values.len() as f64;
    let std = variance.sqrt();
    if std == 0.0 {
        0.0
    } else {
        (value - m) / std
    }
}

fn hurst_p_value(hurst: f64, n: usize) -> f64 {
    if n < 2 {
        return 1.0;
    }
    // Standard error of the Hurst estimate under the null H = 0.5.
    let se = 1.0 / (n as f64).sqrt();
    let z = (hurst - 0.5) / se;
    let normal = match Normal::new(0.0, 1.0) {
        Ok(n) => n,
        Err(_) => return 1.0,
    };
    let two_sided = 2.0 * (1.0 - normal.cdf(z.abs()));
    two_sided.clamp(0.0, 1.0)
}
