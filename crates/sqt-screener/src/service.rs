//! Screener service that combines fundamental and technical filters.

use std::collections::HashMap;

use rust_decimal::prelude::ToPrimitive;
use sqt_core::{Ohlcv, QuantError, Result};
use sqt_indicators::IndicatorCalculator;

use crate::fundamental::{FundamentalData, FundamentalFilter};
use crate::provider::FundamentalProvider;

/// Comparison operator used by [`IndicatorFilter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparator {
    /// Less than.
    Lt,
    /// Greater than.
    Gt,
}

/// A filter that compares the latest value of a technical indicator against a
/// threshold.
#[derive(Debug, Clone, PartialEq)]
pub struct IndicatorFilter {
    /// Indicator name as accepted by [`IndicatorCalculator`] (e.g. `"rsi"`).
    pub indicator: String,
    /// Indicator parameters passed to [`IndicatorCalculator`].
    pub params: HashMap<String, String>,
    /// Comparison to apply against `threshold`.
    pub comparator: Comparator,
    /// Threshold value.
    pub threshold: f64,
}

impl IndicatorFilter {
    /// Returns `true` if `value` satisfies this filter condition.
    pub fn apply(&self, value: f64) -> bool {
        match self.comparator {
            Comparator::Lt => value < self.threshold,
            Comparator::Gt => value > self.threshold,
        }
    }
}

/// High-level entry point for screening securities.
pub struct ScreenerService<P: FundamentalProvider> {
    provider: P,
}

impl<P: FundamentalProvider> ScreenerService<P> {
    /// Creates a new screener backed by `provider`.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    /// Applies a set of fundamental filters to the provider's universe.
    ///
    /// A security passes only if it satisfies **all** supplied filters.
    pub fn screen(&self, filters: &[FundamentalFilter]) -> Result<Vec<FundamentalData>> {
        Ok(self
            .provider
            .get_all()?
            .into_iter()
            .filter(|data| filters.iter().all(|f| f.apply(data)))
            .collect())
    }

    /// Applies fundamental filters and then filters by technical indicators.
    ///
    /// `ohlcv_data` maps tickers to their OHLCV price history. For every ticker
    /// that passes the fundamental filters, each indicator filter is evaluated
    /// against the most recent non-`None` value produced by
    /// [`IndicatorCalculator`]. A ticker passes only if all indicator filters
    /// are satisfied. Tickers without OHLCV data or without a warm indicator
    /// value are excluded.
    pub fn screen_with_indicators(
        &self,
        filters: &[FundamentalFilter],
        indicator_filters: &[IndicatorFilter],
        ohlcv_data: &HashMap<String, Vec<Ohlcv>>,
    ) -> Result<Vec<FundamentalData>> {
        let fundamental_matches = self.screen(filters)?;
        let mut out = Vec::with_capacity(fundamental_matches.len());

        for data in fundamental_matches {
            let series = match ohlcv_data.get(&data.ticker) {
                Some(s) if s.len() >= 2 => s,
                _ => continue,
            };

            let mut passes = true;
            for filter in indicator_filters {
                match latest_indicator_value(
                    &data.ticker,
                    series,
                    &filter.indicator,
                    &filter.params,
                )? {
                    Some(v) if filter.apply(v) => {}
                    _ => passes = false,
                }
            }
            if passes {
                out.push(data);
            }
        }

        Ok(out)
    }
}

fn latest_indicator_value(
    ticker: &str,
    series: &[Ohlcv],
    indicator: &str,
    params: &HashMap<String, String>,
) -> Result<Option<f64>> {
    if series.is_empty() {
        return Ok(None);
    }
    let result = IndicatorCalculator::calculate(indicator, series, params)?;
    let latest = result.values.iter().rev().find_map(|(_, v)| *v);
    match latest {
        Some(dec) => Ok(Some(dec.to_f64().ok_or_else(|| {
            QuantError::DataQuality(format!(
                "indicator value for `{ticker}` cannot be represented as f64"
            ))
        })?)),
        None => Ok(None),
    }
}
