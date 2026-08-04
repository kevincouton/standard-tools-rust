//! Relative strength index indicator.

use crate::calculator::IndicatorResult;
use crate::indicators::{decimal_mean, parse_param_usize};
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, Result};
use std::collections::HashMap;

/// Calculate the relative strength index.
///
/// Uses the standard Wilder smoothing: the average gain/loss is initialised
/// with the simple mean of the first `period` price changes and then smoothed
/// with `period` thereafter. Values are `None` until `period` price changes
/// have been observed, i.e. the first `period` bars.
pub fn calculate(series: &[Ohlcv], params: &HashMap<String, String>) -> Result<IndicatorResult> {
    let period = parse_param_usize(params, "period", 14)?;
    let mut values = Vec::with_capacity(series.len());

    if period == 0 || series.len() < period + 1 {
        values.extend(series.iter().map(|bar| (bar.date, None)));
        return Ok(IndicatorResult {
            name: "rsi".to_string(),
            params: params.clone(),
            values,
            extra_series: HashMap::new(),
        });
    }

    let mut gains = Vec::with_capacity(period);
    let mut losses = Vec::with_capacity(period);

    for window in series.windows(2).take(period) {
        let diff = window[1].close - window[0].close;
        if diff >= Decimal::ZERO {
            gains.push(diff);
            losses.push(Decimal::ZERO);
        } else {
            gains.push(Decimal::ZERO);
            losses.push(-diff);
        }
    }

    let mut avg_gain = decimal_mean(&gains).expect("gains is non-empty");
    let mut avg_loss = decimal_mean(&losses).expect("losses is non-empty");

    for bar in series.iter().take(period) {
        values.push((bar.date, None));
    }
    values.push((series[period].date, Some(rsi_value(avg_gain, avg_loss))));

    let smoothing = Decimal::from(period as i64);
    let smoothing_minus_one = Decimal::from(period as i64 - 1);

    for window in series.windows(2).skip(period) {
        let diff = window[1].close - window[0].close;
        let gain = if diff >= Decimal::ZERO {
            diff
        } else {
            Decimal::ZERO
        };
        let loss = if diff >= Decimal::ZERO {
            Decimal::ZERO
        } else {
            -diff
        };

        avg_gain = (avg_gain * smoothing_minus_one + gain) / smoothing;
        avg_loss = (avg_loss * smoothing_minus_one + loss) / smoothing;

        values.push((window[1].date, Some(rsi_value(avg_gain, avg_loss))));
    }

    Ok(IndicatorResult {
        name: "rsi".to_string(),
        params: params.clone(),
        values,
        extra_series: HashMap::new(),
    })
}

fn rsi_value(avg_gain: Decimal, avg_loss: Decimal) -> Decimal {
    if avg_loss == Decimal::ZERO {
        Decimal::from(100)
    } else {
        let rs = avg_gain / avg_loss;
        Decimal::from(100) - (Decimal::from(100) / (Decimal::ONE + rs))
    }
}
