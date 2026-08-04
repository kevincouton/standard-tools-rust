//! Average true range indicator.

use crate::calculator::IndicatorResult;
use crate::indicators::{decimal_mean, parse_param_usize};
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, Result};
use std::collections::HashMap;

/// Calculate the average true range.
///
/// Uses Wilder's smoothing after initialising with the simple mean of the
/// first `period` true ranges. The first `period` values are `None`.
pub fn calculate(series: &[Ohlcv], params: &HashMap<String, String>) -> Result<IndicatorResult> {
    let period = parse_param_usize(params, "period", 14)?;
    let mut values = Vec::with_capacity(series.len());

    if period == 0 || series.len() < period + 1 {
        values.extend(series.iter().map(|bar| (bar.date, None)));
        return Ok(IndicatorResult {
            name: "atr".to_string(),
            params: params.clone(),
            values,
            extra_series: HashMap::new(),
        });
    }

    let mut initial_trs = Vec::with_capacity(period);
    for window in series.windows(2).take(period) {
        initial_trs.push(true_range(&window[1], &window[0]));
    }

    let mut atr = decimal_mean(&initial_trs).expect("initial_trs is non-empty");
    let smoothing = Decimal::from(period as i64);
    let smoothing_minus_one = Decimal::from(period as i64 - 1);

    for bar in series.iter().take(period) {
        values.push((bar.date, None));
    }
    values.push((series[period].date, Some(atr)));

    for window in series.windows(2).skip(period) {
        let tr = true_range(&window[1], &window[0]);
        atr = (atr * smoothing_minus_one + tr) / smoothing;
        values.push((window[1].date, Some(atr)));
    }

    Ok(IndicatorResult {
        name: "atr".to_string(),
        params: params.clone(),
        values,
        extra_series: HashMap::new(),
    })
}

fn true_range(bar: &Ohlcv, prev: &Ohlcv) -> Decimal {
    let high_low = bar.high - bar.low;
    let high_close = (bar.high - prev.close).abs();
    let low_close = (bar.low - prev.close).abs();
    high_low.max(high_close).max(low_close)
}
