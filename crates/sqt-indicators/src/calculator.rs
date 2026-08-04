//! Indicator calculator and result type.

use crate::indicators;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, QuantError, Result};
use std::collections::HashMap;

/// The outcome of a single indicator calculation.
///
/// The `values` vector is aligned with the input OHLCV series: each tuple
/// contains the bar date and the indicator value, if one is available. Values
/// are `None` during the indicator's warming period.
///
/// Multi-series indicators (for example Bollinger Bands or MACD) store their
/// additional output in `extra_series`, keyed by series name. The primary
/// `values` field always contains the main series.
#[derive(Debug, Clone, PartialEq)]
pub struct IndicatorResult {
    /// Name of the indicator that produced this result.
    pub name: String,

    /// Parameters used to configure the indicator.
    pub params: HashMap<String, String>,

    /// Date-aligned indicator values.
    pub values: Vec<(NaiveDate, Option<Decimal>)>,

    /// Additional named series produced by the indicator.
    pub extra_series: HashMap<String, Vec<(NaiveDate, Option<Decimal>)>>,
}

/// Unified entry point for calculating technical indicators.
///
/// Dispatch is performed by indicator name. Unknown names produce
/// [`QuantError::InvalidCommand`].
pub struct IndicatorCalculator;

impl IndicatorCalculator {
    /// Calculate the named indicator over `series` using `params`.
    ///
    /// # Supported indicators
    ///
    /// * `sma` — simple moving average of close prices (`period`, default 20)
    /// * `ema` — exponential moving average of close prices (`period`, default 20)
    /// * `rsi` — relative strength index (`period`, default 14)
    /// * `macd` — MACD line (`fast` 12, `slow` 26, `signal` 9)
    /// * `bollinger_bands` — middle band / SMA (`period` 20, `std_dev` 2)
    /// * `atr` — average true range (`period`, default 14)
    /// * `obv` — on-balance volume
    /// * `vwap` — volume-weighted average price
    ///
    /// # Errors
    ///
    /// Returns [`QuantError::InvalidCommand`] if `name` is unknown or if any
    /// parameter cannot be parsed.
    pub fn calculate(
        name: &str,
        series: &[Ohlcv],
        params: &HashMap<String, String>,
    ) -> Result<IndicatorResult> {
        match name {
            "sma" => indicators::sma::calculate(series, params),
            "ema" => indicators::ema::calculate(series, params),
            "rsi" => indicators::rsi::calculate(series, params),
            "macd" => indicators::macd::calculate(series, params),
            "bollinger_bands" => indicators::bollinger::calculate(series, params),
            "atr" => indicators::atr::calculate(series, params),
            "obv" => indicators::obv::calculate(series, params),
            "vwap" => indicators::vwap::calculate(series, params),
            _ => Err(QuantError::InvalidCommand(format!(
                "unknown indicator: {name}"
            ))),
        }
    }
}
