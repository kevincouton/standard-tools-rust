//! On-balance volume indicator.

use crate::calculator::IndicatorResult;
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, Result};
use std::collections::HashMap;

/// Calculate on-balance volume.
///
/// OBV starts at the volume of the first bar and is adjusted by subsequent
/// volume depending on whether the close price rose, fell, or was unchanged
/// relative to the previous bar. There is no warming period.
pub fn calculate(series: &[Ohlcv], params: &HashMap<String, String>) -> Result<IndicatorResult> {
    let mut values = Vec::with_capacity(series.len());

    if series.is_empty() {
        return Ok(IndicatorResult {
            name: "obv".to_string(),
            params: params.clone(),
            values,
            extra_series: HashMap::new(),
        });
    }

    let mut obv = Decimal::from(series[0].volume);
    values.push((series[0].date, Some(obv)));

    for window in series.windows(2) {
        let current_close = window[1].close;
        let previous_close = window[0].close;
        let volume = Decimal::from(window[1].volume);

        if current_close > previous_close {
            obv += volume;
        } else if current_close < previous_close {
            obv -= volume;
        }

        values.push((window[1].date, Some(obv)));
    }

    Ok(IndicatorResult {
        name: "obv".to_string(),
        params: params.clone(),
        values,
        extra_series: HashMap::new(),
    })
}
