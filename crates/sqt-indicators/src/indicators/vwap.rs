//! Volume-weighted average price indicator.

use crate::calculator::IndicatorResult;
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, Result};
use std::collections::HashMap;

/// Calculate the volume-weighted average price.
///
/// VWAP uses the typical price `(high + low + close) / 3` weighted by volume
/// over the entire series provided. There is no warming period.
pub fn calculate(series: &[Ohlcv], params: &HashMap<String, String>) -> Result<IndicatorResult> {
    let mut values = Vec::with_capacity(series.len());
    let mut cumulative_tp_volume = Decimal::ZERO;
    let mut cumulative_volume = Decimal::ZERO;

    for bar in series {
        let typical_price = (bar.high + bar.low + bar.close) / Decimal::from(3);
        let volume = Decimal::from(bar.volume);

        cumulative_tp_volume += typical_price * volume;
        cumulative_volume += volume;

        let vwap = if cumulative_volume == Decimal::ZERO {
            None
        } else {
            Some(cumulative_tp_volume / cumulative_volume)
        };

        values.push((bar.date, vwap));
    }

    Ok(IndicatorResult {
        name: "vwap".to_string(),
        params: params.clone(),
        values,
        extra_series: HashMap::new(),
    })
}
