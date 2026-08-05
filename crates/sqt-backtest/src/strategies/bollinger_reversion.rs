//! Bollinger Bands mean-reversion strategy.

use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, QuantError, Result};
use sqt_indicators::IndicatorCalculator;

use crate::signal::{Signal, SignalResult};
use crate::Strategy;

/// A strategy that buys when price falls below the lower Bollinger Band and
/// sells when price rises above the upper band.
#[derive(Debug, Clone, Copy, Default)]
pub struct BollingerReversion;

impl Strategy for BollingerReversion {
    fn name(&self) -> &'static str {
        "bollinger_reversion"
    }

    fn signals(
        &self,
        series: &[Ohlcv],
        params: &HashMap<String, String>,
    ) -> Result<Vec<SignalResult>> {
        let bb_result = IndicatorCalculator::calculate("bollinger_bands", series, params)?;
        let upper_band: Vec<(NaiveDate, Option<Decimal>)> = bb_result
            .extra_series
            .get("upper")
            .cloned()
            .unwrap_or_default();
        let lower_band: Vec<(NaiveDate, Option<Decimal>)> = bb_result
            .extra_series
            .get("lower")
            .cloned()
            .unwrap_or_default();

        if upper_band.len() != series.len() || lower_band.len() != series.len() {
            return Err(QuantError::DataQuality(
                "indicator output length does not match input series".to_string(),
            ));
        }

        let mut signals = Vec::with_capacity(series.len());
        for (i, bar) in series.iter().enumerate() {
            let signal = match (upper_band[i].1, lower_band[i].1) {
                (Some(upper), Some(lower)) => {
                    if bar.close < lower {
                        Signal::Buy
                    } else if bar.close > upper {
                        Signal::Sell
                    } else {
                        Signal::Hold
                    }
                }
                _ => Signal::Hold,
            };
            signals.push(SignalResult {
                date: bar.date,
                signal,
            });
        }

        Ok(signals)
    }
}
