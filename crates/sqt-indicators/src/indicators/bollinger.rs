//! Bollinger Bands indicator.

use crate::calculator::IndicatorResult;
use crate::indicators::{decimal_mean, parse_param_usize};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, Result};
use std::collections::HashMap;

/// Calculate Bollinger Bands.
///
/// The primary `values` series contains the middle band (simple moving average
/// of close prices). The `extra_series` map contains the `upper` and `lower`
/// bands, computed as the middle band plus/minus `std_dev` times the rolling
/// standard deviation of close prices. All three series are `None` during the
/// warming period.
pub fn calculate(series: &[Ohlcv], params: &HashMap<String, String>) -> Result<IndicatorResult> {
    let period = parse_param_usize(params, "period", 20)?;
    let std_dev = parse_param_usize(params, "std_dev", 2)?;

    let mut values = Vec::with_capacity(series.len());
    let mut upper = Vec::with_capacity(series.len());
    let mut lower = Vec::with_capacity(series.len());

    if period == 0 || series.len() < period {
        let none_values: Vec<_> = series.iter().map(|bar| (bar.date, None)).collect();
        values.extend(none_values.iter().copied());
        upper.extend(none_values.iter().copied());
        lower.extend(none_values.iter().copied());
        return Ok(IndicatorResult {
            name: "bollinger_bands".to_string(),
            params: params.clone(),
            values,
            extra_series: HashMap::from([
                ("upper".to_string(), upper),
                ("lower".to_string(), lower),
            ]),
        });
    }

    for (i, bar) in series.iter().enumerate() {
        if i + 1 < period {
            values.push((bar.date, None));
            upper.push((bar.date, None));
            lower.push((bar.date, None));
        } else {
            let window: Vec<_> = series[i + 1 - period..=i]
                .iter()
                .map(|bar| bar.close)
                .collect();
            let middle = decimal_mean(&window).expect("window is non-empty");
            let band_width = Decimal::from_f64(std_dev as f64 * sample_std_dev(&window))
                .unwrap_or(Decimal::ZERO);

            values.push((bar.date, Some(middle)));
            upper.push((bar.date, Some(middle + band_width)));
            lower.push((bar.date, Some(middle - band_width)));
        }
    }

    Ok(IndicatorResult {
        name: "bollinger_bands".to_string(),
        params: params.clone(),
        values,
        extra_series: HashMap::from([("upper".to_string(), upper), ("lower".to_string(), lower)]),
    })
}

/// Sample standard deviation of a non-empty slice of decimals.
///
/// Uses sample variance (divide by `n - 1`), the conventional calculation for
/// Bollinger Bands. Computed via `f64` because `Decimal` does not enable the
/// `maths` feature in this workspace.
fn sample_std_dev(values: &[Decimal]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mean: f64 = values.iter().filter_map(|d| d.to_f64()).sum::<f64>() / values.len() as f64;
    let variance: f64 = values
        .iter()
        .filter_map(|d| d.to_f64())
        .map(|v| (v - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    variance.sqrt()
}
