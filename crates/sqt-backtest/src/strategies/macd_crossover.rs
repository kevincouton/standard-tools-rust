//! MACD crossover strategy.

use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, QuantError, Result};
use sqt_indicators::IndicatorCalculator;

use crate::signal::{Signal, SignalResult};
use crate::Strategy;

/// A strategy that goes long when the MACD line crosses above its signal line
/// and short when it crosses below.
#[derive(Debug, Clone, Copy, Default)]
pub struct MacdCrossover;

impl Strategy for MacdCrossover {
    fn name(&self) -> &'static str {
        "macd_crossover"
    }

    fn signals(
        &self,
        series: &[Ohlcv],
        params: &HashMap<String, String>,
    ) -> Result<Vec<SignalResult>> {
        let macd_result = IndicatorCalculator::calculate("macd", series, params)?;
        let macd_line: Vec<(NaiveDate, Option<Decimal>)> = macd_result.values;
        let signal_line: Vec<(NaiveDate, Option<Decimal>)> = macd_result
            .extra_series
            .get("signal")
            .cloned()
            .unwrap_or_default();

        if macd_line.len() != series.len() || signal_line.len() != series.len() {
            return Err(QuantError::DataQuality(
                "indicator output length does not match input series".to_string(),
            ));
        }

        let mut signals = Vec::with_capacity(series.len());
        for i in 0..series.len() {
            let signal = match (macd_line[i].1, signal_line[i].1) {
                (Some(macd), Some(sig)) => {
                    if i == 0 {
                        if macd > sig {
                            Signal::Buy
                        } else if macd < sig {
                            Signal::Sell
                        } else {
                            Signal::Hold
                        }
                    } else {
                        match (macd_line[i - 1].1, signal_line[i - 1].1) {
                            (Some(prev_macd), Some(prev_sig)) => {
                                if macd > sig && prev_macd <= prev_sig {
                                    Signal::Buy
                                } else if macd < sig && prev_macd >= prev_sig {
                                    Signal::Sell
                                } else {
                                    Signal::Hold
                                }
                            }
                            _ => Signal::Hold,
                        }
                    }
                }
                _ => Signal::Hold,
            };
            signals.push(SignalResult {
                date: series[i].date,
                signal,
            });
        }

        Ok(signals)
    }
}
