//! Hurst exponent estimation using rescaled range (R/S) analysis.
//!
//! The Hurst exponent characterises the long-term memory of a time series:
//!
//! * `H < 0.5` — mean-reverting behaviour.
//! * `H ≈ 0.5` — geometric random walk (no long-term memory).
//! * `H > 0.5` — trending/persistent behaviour.
//!
//! This implementation converts prices to log-returns and then performs the
//! classic R/S analysis over a range of lags. The exponent is the slope of the
//! log-log regression of the average rescaled range on the lag length.

use sqt_core::{QuantError, Result};

use crate::math::{mean, validate_finite};

/// Result of a Hurst exponent estimation.
#[derive(Debug, Clone, PartialEq)]
pub struct HurstResult {
    /// Estimated Hurst exponent, typically in the range `[0, 1]`.
    pub exponent: f64,
    /// Human-readable interpretation of the exponent.
    pub interpretation: String,
}

/// Estimates the Hurst exponent of a price series using R/S analysis.
///
/// `prices` must contain at least 50 observations. Log-returns are computed
/// internally, so the input should be raw price levels. `max_lag` controls the
/// largest lag used in the R/S regression; when `None`, it defaults to one
/// quarter of the return sample size.
///
/// # Errors
///
/// Returns [`QuantError::DataQuality`] if the input is too short, contains
/// non-positive prices, non-finite values, or if the computed lag range is
/// degenerate.
pub fn hurst_exponent(prices: &[f64], max_lag: Option<usize>) -> Result<HurstResult> {
    validate_finite(prices)?;
    if prices.len() < 50 {
        return Err(QuantError::DataQuality(
            "Hurst estimation requires at least 50 price observations".to_string(),
        ));
    }
    if prices.iter().any(|&p| p <= 0.0) {
        return Err(QuantError::DataQuality(
            "prices must be positive to compute log-returns".to_string(),
        ));
    }

    let returns: Vec<f64> = prices.windows(2).map(|w| (w[1] / w[0]).ln()).collect();
    let n = returns.len();

    let max_l = max_lag
        .map(|m| m.clamp(8, n / 4))
        .unwrap_or_else(|| (n / 4).max(8));

    rs_log_log(&returns, 8, max_l)
}

/// R/S analysis applied directly to the input levels (no log-return transform).
///
/// This is intended as an internal stationarity proxy for cointegration
/// residuals, which may be negative. For very short series the estimate is
/// unreliable, so the random-walk null value `0.5` is returned as a fallback.
pub(crate) fn hurst_exponent_rs_levels(levels: &[f64]) -> f64 {
    if levels.len() < 10 {
        return 0.5;
    }
    let n = levels.len();
    let max_l = (n / 2).max(4).min(n.saturating_sub(1));
    rs_log_log(levels, 2, max_l)
        .map(|r| r.exponent)
        .unwrap_or(0.5)
}

fn rs_log_log(series: &[f64], min_lag: usize, max_lag: usize) -> Result<HurstResult> {
    let n = series.len();
    let max_l = max_lag.clamp(min_lag, n.saturating_sub(1));
    if max_l <= min_lag {
        return Err(QuantError::DataQuality(
            "series is too short for meaningful R/S analysis".to_string(),
        ));
    }

    // Use enough lags to obtain a stable slope estimate without over-fitting.
    let step = ((max_l - min_lag) / 50).max(1);
    let mut log_lags = Vec::new();
    let mut log_rs = Vec::new();

    for lag in (min_lag..=max_l).step_by(step) {
        let rs = rescaled_range(series, lag);
        if rs > 0.0 {
            log_lags.push((lag as f64).ln());
            log_rs.push(rs.ln());
        }
    }

    if log_lags.len() < 2 {
        return Err(QuantError::DataQuality(
            "could not compute rescaled range for any lag".to_string(),
        ));
    }

    let exponent = slope(&log_lags, &log_rs);
    let exponent = exponent.clamp(0.0, 1.0);

    Ok(HurstResult {
        exponent,
        interpretation: interpret(exponent),
    })
}

fn rescaled_range(series: &[f64], lag: usize) -> f64 {
    let chunks: Vec<&[f64]> = series.chunks(lag).collect();
    if chunks.is_empty() {
        return 0.0;
    }

    let mut total = 0.0;
    let mut count = 0usize;

    for chunk in chunks {
        if chunk.len() < 2 {
            continue;
        }
        let chunk_mean = mean(chunk);
        let mut cumulative = 0.0;
        let mut max_dev = f64::NEG_INFINITY;
        let mut min_dev = f64::INFINITY;
        for &r in chunk {
            cumulative += r - chunk_mean;
            max_dev = max_dev.max(cumulative);
            min_dev = min_dev.min(cumulative);
        }
        let range = max_dev - min_dev;
        let variance =
            chunk.iter().map(|&r| (r - chunk_mean).powi(2)).sum::<f64>() / chunk.len() as f64;
        let std = variance.sqrt();
        if std > 0.0 {
            total += range / std;
            count += 1;
        }
    }

    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

fn slope(xs: &[f64], ys: &[f64]) -> f64 {
    let mean_x = mean(xs);
    let mean_y = mean(ys);

    let mut num = 0.0;
    let mut den = 0.0;
    for (&x, &y) in xs.iter().zip(ys.iter()) {
        let dx = x - mean_x;
        num += dx * (y - mean_y);
        den += dx * dx;
    }

    if den == 0.0 {
        0.0
    } else {
        num / den
    }
}

fn interpret(exponent: f64) -> String {
    if exponent < 0.4 {
        "mean-reverting".to_string()
    } else if exponent < 0.55 {
        "approximately random walk".to_string()
    } else {
        "persistent/trending".to_string()
    }
}
