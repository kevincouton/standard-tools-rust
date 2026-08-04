//! Moving average convergence/divergence indicator.

use crate::calculator::IndicatorResult;
use crate::indicators::decimal_mean;
use crate::indicators::ema::ema_values;
use crate::indicators::parse_param_usize;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, Result};
use std::collections::HashMap;

/// Calculate MACD.
///
/// The primary `values` series contains the MACD line (fast EMA minus slow
/// EMA). The `extra_series` map contains the `signal` line (EMA of the MACD
/// line with period `signal`) and the `histogram` (MACD minus signal).
pub fn calculate(series: &[Ohlcv], params: &HashMap<String, String>) -> Result<IndicatorResult> {
    let fast = parse_param_usize(params, "fast", 12)?;
    let slow = parse_param_usize(params, "slow", 26)?;
    let signal = parse_param_usize(params, "signal", 9)?;

    let mut values = Vec::with_capacity(series.len());
    let mut signal_series = Vec::with_capacity(series.len());
    let mut histogram_series = Vec::with_capacity(series.len());

    if fast == 0 || slow == 0 || signal == 0 || series.len() < slow {
        let none_values: Vec<_> = series.iter().map(|bar| (bar.date, None)).collect();
        values.extend(none_values.iter().copied());
        signal_series.extend(none_values.iter().copied());
        histogram_series.extend(none_values.iter().copied());
        return Ok(IndicatorResult {
            name: "macd".to_string(),
            params: params.clone(),
            values,
            extra_series: HashMap::from([
                ("signal".to_string(), signal_series),
                ("histogram".to_string(), histogram_series),
            ]),
        });
    }

    let fast_ema = ema_values(series, fast);
    let slow_ema = ema_values(series, slow);

    // MACD line = fast EMA - slow EMA, aligned by date.
    let mut macd_line: Vec<(NaiveDate, Option<Decimal>)> = Vec::with_capacity(series.len());
    for ((_, fast_val), (date, slow_val)) in fast_ema.iter().zip(slow_ema.iter()) {
        let value = match (fast_val, slow_val) {
            (Some(f), Some(s)) => Some(*f - *s),
            _ => None,
        };
        macd_line.push((*date, value));
    }

    // Signal line = EMA of MACD line values.
    let signal_ema = ema_of_options(&macd_line, signal);

    for i in 0..series.len() {
        let date = series[i].date;
        let macd = macd_line[i].1;
        let sig = signal_ema[i].1;
        let histogram = match (macd, sig) {
            (Some(m), Some(s)) => Some(m - s),
            _ => None,
        };

        values.push((date, macd));
        signal_series.push((date, sig));
        histogram_series.push((date, histogram));
    }

    Ok(IndicatorResult {
        name: "macd".to_string(),
        params: params.clone(),
        values,
        extra_series: HashMap::from([
            ("signal".to_string(), signal_series),
            ("histogram".to_string(), histogram_series),
        ]),
    })
}

/// Compute an EMA over a series of optional values.
///
/// `None` entries are skipped for EMA seeding and propagation; the result has
/// `None` where no EMA value is available yet.
fn ema_of_options(
    series: &[(NaiveDate, Option<Decimal>)],
    period: usize,
) -> Vec<(NaiveDate, Option<Decimal>)> {
    let mut result = Vec::with_capacity(series.len());
    if period == 0 {
        result.extend(series.iter().map(|(date, _)| (*date, None)));
        return result;
    }

    let multiplier = Decimal::from(2) / Decimal::from(period as i64 + 1);
    let mut ema: Option<Decimal> = None;
    let mut seen = 0usize;
    let mut seed_values = Vec::with_capacity(period);

    for (date, value) in series {
        match (value, ema) {
            (Some(v), None) => {
                seen += 1;
                seed_values.push(*v);
                if seen == period {
                    let seed = decimal_mean(&seed_values).expect("seed window is non-empty");
                    ema = Some(seed);
                    result.push((*date, Some(seed)));
                } else {
                    result.push((*date, None));
                }
            }
            (Some(v), Some(current)) => {
                let next = (*v - current) * multiplier + current;
                ema = Some(next);
                result.push((*date, Some(next)));
            }
            (None, _) => {
                result.push((*date, None));
            }
        }
    }

    result
}
