//! Simple moving average crossover strategy.

use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, QuantError, Result};
use sqt_indicators::IndicatorCalculator;

use crate::signal::{Signal, SignalResult};
use crate::Strategy;

/// A strategy that goes long when a fast SMA crosses above a slow SMA and short
/// when it crosses below.
#[derive(Debug, Clone, Copy, Default)]
pub struct SmaCrossover;

impl Strategy for SmaCrossover {
    fn name(&self) -> &'static str {
        "sma_crossover"
    }

    fn signals(
        &self,
        series: &[Ohlcv],
        params: &HashMap<String, String>,
    ) -> Result<Vec<SignalResult>> {
        let mut fast_params = params.clone();
        fast_params.insert(
            "period".to_string(),
            params
                .get("fast")
                .cloned()
                .unwrap_or_else(|| "10".to_string()),
        );
        let mut slow_params = params.clone();
        slow_params.insert(
            "period".to_string(),
            params
                .get("slow")
                .cloned()
                .unwrap_or_else(|| "30".to_string()),
        );

        let fast_result = IndicatorCalculator::calculate("sma", series, &fast_params)?;
        let slow_result = IndicatorCalculator::calculate("sma", series, &slow_params)?;

        let fast_values: Vec<(NaiveDate, Option<Decimal>)> = fast_result.values;
        let slow_values: Vec<(NaiveDate, Option<Decimal>)> = slow_result.values;

        if fast_values.len() != series.len() || slow_values.len() != series.len() {
            return Err(QuantError::DataQuality(
                "indicator output length does not match input series".to_string(),
            ));
        }

        let mut signals = Vec::with_capacity(series.len());
        for i in 0..series.len() {
            let signal = match (fast_values[i].1, slow_values[i].1) {
                (Some(fast), Some(slow)) => {
                    if i == 0 {
                        if fast > slow {
                            Signal::Buy
                        } else if fast < slow {
                            Signal::Sell
                        } else {
                            Signal::Hold
                        }
                    } else {
                        match (fast_values[i - 1].1, slow_values[i - 1].1) {
                            (Some(prev_fast), Some(prev_slow)) => {
                                if fast > slow && prev_fast <= prev_slow {
                                    Signal::Buy
                                } else if fast < slow && prev_fast >= prev_slow {
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
