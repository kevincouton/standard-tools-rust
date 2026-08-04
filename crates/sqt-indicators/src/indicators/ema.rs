//! Exponential moving average indicator.

use crate::calculator::IndicatorResult;
use crate::indicators::{decimal_mean, parse_param_usize};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, Result};
use std::collections::HashMap;

/// Calculate the exponential moving average of close prices.
///
/// The EMA is seeded with the SMA over the first `period` bars. The first
/// `period - 1` values are `None`.
pub fn calculate(series: &[Ohlcv], params: &HashMap<String, String>) -> Result<IndicatorResult> {
    let period = parse_param_usize(params, "period", 20)?;
    let mut values = Vec::with_capacity(series.len());

    if period == 0 || series.len() < period {
        values.extend(series.iter().map(|bar| (bar.date, None)));
        return Ok(IndicatorResult {
            name: "ema".to_string(),
            params: params.clone(),
            values,
            extra_series: HashMap::new(),
        });
    }

    let multiplier = Decimal::from(2) / Decimal::from(period as i64 + 1);
    let mut ema = Decimal::ZERO;

    for (i, bar) in series.iter().enumerate() {
        if i + 1 < period {
            values.push((bar.date, None));
        } else if i + 1 == period {
            let window: Vec<_> = series[0..=i].iter().map(|bar| bar.close).collect();
            ema = decimal_mean(&window).expect("window is non-empty");
            values.push((bar.date, Some(ema)));
        } else {
            ema = (bar.close - ema) * multiplier + ema;
            values.push((bar.date, Some(ema)));
        }
    }

    Ok(IndicatorResult {
        name: "ema".to_string(),
        params: params.clone(),
        values,
        extra_series: HashMap::new(),
    })
}

/// Compute raw EMA values for internal reuse (e.g. by MACD).
///
/// Entries before the EMA has warmed up are `None`; warmed values are
/// `Some(...)`. This avoids returning misleading sentinel zeros for unready
/// bars.
pub(crate) fn ema_values(series: &[Ohlcv], period: usize) -> Vec<(NaiveDate, Option<Decimal>)> {
    let mut result = Vec::with_capacity(series.len());
    if period == 0 || series.len() < period {
        result.extend(series.iter().map(|bar| (bar.date, None)));
        return result;
    }

    let multiplier = Decimal::from(2) / Decimal::from(period as i64 + 1);
    let mut ema = Decimal::ZERO;

    for (i, bar) in series.iter().enumerate() {
        if i + 1 < period {
            result.push((bar.date, None));
        } else if i + 1 == period {
            let window: Vec<_> = series[0..=i].iter().map(|bar| bar.close).collect();
            ema = decimal_mean(&window).expect("window is non-empty");
            result.push((bar.date, Some(ema)));
        } else {
            ema = (bar.close - ema) * multiplier + ema;
            result.push((bar.date, Some(ema)));
        }
    }

    result
}
