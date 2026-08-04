//! Simple moving average indicator.

use crate::calculator::IndicatorResult;
use crate::indicators::{decimal_mean, parse_param_usize};
use sqt_core::{Ohlcv, Result};
use std::collections::HashMap;

/// Calculate the simple moving average of close prices.
///
/// The first `period - 1` values are `None` because the moving window is not
/// yet full.
pub fn calculate(series: &[Ohlcv], params: &HashMap<String, String>) -> Result<IndicatorResult> {
    let period = parse_param_usize(params, "period", 20)?;
    let mut values = Vec::with_capacity(series.len());

    if period == 0 || series.len() < period {
        values.extend(series.iter().map(|bar| (bar.date, None)));
        return Ok(IndicatorResult {
            name: "sma".to_string(),
            params: params.clone(),
            values,
            extra_series: HashMap::new(),
        });
    }

    for (i, bar) in series.iter().enumerate() {
        if i + 1 < period {
            values.push((bar.date, None));
        } else {
            let window: Vec<_> = series[i + 1 - period..=i]
                .iter()
                .map(|bar| bar.close)
                .collect();
            values.push((bar.date, decimal_mean(&window)));
        }
    }

    Ok(IndicatorResult {
        name: "sma".to_string(),
        params: params.clone(),
        values,
        extra_series: HashMap::new(),
    })
}
