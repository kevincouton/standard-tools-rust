//! RSI mean-reversion strategy.

use std::collections::HashMap;

use rust_decimal::Decimal;
use sqt_core::{Ohlcv, QuantError, Result};
use sqt_indicators::IndicatorCalculator;

use crate::signal::{Signal, SignalResult};
use crate::Strategy;

/// A strategy that buys when RSI is oversold and sells when it is overbought.
#[derive(Debug, Clone, Copy, Default)]
pub struct RsiMeanReversion;

impl Strategy for RsiMeanReversion {
    fn name(&self) -> &'static str {
        "rsi_mean_reversion"
    }

    fn signals(
        &self,
        series: &[Ohlcv],
        params: &HashMap<String, String>,
    ) -> Result<Vec<SignalResult>> {
        let oversold = parse_decimal_param(params, "oversold", Decimal::from(30))?;
        let overbought = parse_decimal_param(params, "overbought", Decimal::from(70))?;

        let rsi_result = IndicatorCalculator::calculate("rsi", series, params)?;

        let mut signals = Vec::with_capacity(series.len());
        for (date, value) in &rsi_result.values {
            let signal = match value {
                Some(rsi) if *rsi < oversold => Signal::Buy,
                Some(rsi) if *rsi > overbought => Signal::Sell,
                _ => Signal::Hold,
            };
            signals.push(SignalResult {
                date: *date,
                signal,
            });
        }

        // The first value is ignored because RSI uses period price changes; ensure
        // we never emit a trade on the very first bar.
        if !signals.is_empty() {
            signals[0].signal = Signal::Hold;
        }

        Ok(signals)
    }
}

fn parse_decimal_param(
    params: &HashMap<String, String>,
    key: &str,
    default: Decimal,
) -> Result<Decimal> {
    match params.get(key) {
        Some(value) => value
            .parse::<Decimal>()
            .map_err(|_| QuantError::InvalidCommand(format!("invalid value for {key}: {value}"))),
        None => Ok(default),
    }
}
