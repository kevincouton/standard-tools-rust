//! Domain value objects shared across the Standard Tools workspace.
//!
//! This module defines small, immutable, serialisable types that model common
//! financial-domain concepts: a traded instrument ([`Ticker`]), a calendar date
//! span ([`DateRange`]), a single open/high/low/close/volume bar ([`Ohlcv`]),
//! and a bar aggregation interval ([`BarInterval`]).

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{QuantError, Result};

/// A traded instrument identified by its symbol and optional exchange.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ticker {
    /// The ticker symbol, e.g. `"AAPL"`.
    pub symbol: String,

    /// The exchange on which the instrument trades, e.g. `"NASDAQ"`.
    pub exchange: Option<String>,
}

impl Ticker {
    /// Creates a new ticker with the given symbol and no exchange.
    ///
    /// The symbol is accepted as-is. Use [`Ticker::try_new`] when validation is
    /// required.
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            exchange: None,
        }
    }

    /// Creates a new ticker after validating that the symbol is non-empty and
    /// not whitespace-only.
    ///
    /// # Errors
    ///
    /// Returns [`QuantError::InvalidCommand`] if `symbol` is empty or contains
    /// only whitespace.
    pub fn try_new(symbol: impl Into<String>) -> Result<Self> {
        let symbol = symbol.into();
        if symbol.trim().is_empty() {
            return Err(QuantError::InvalidCommand(format!(
                "ticker symbol must be non-empty, got `{symbol}`"
            )));
        }
        Ok(Self {
            symbol,
            exchange: None,
        })
    }

    /// Sets the exchange for this ticker.
    ///
    /// Because this returns `Self`, the result should be used; callers that
    /// discard it will not observe the exchange change.
    #[must_use]
    pub fn with_exchange(mut self, exchange: impl Into<String>) -> Self {
        self.exchange = Some(exchange.into());
        self
    }
}

/// An inclusive calendar date range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRange {
    /// Start of the range.
    pub start: NaiveDate,

    /// End of the range.
    pub end: NaiveDate,
}

impl DateRange {
    /// Creates a new date range after validating that `start <= end`.
    ///
    /// # Errors
    ///
    /// Returns [`QuantError::InvalidCommand`] if `start` is after `end`.
    pub fn try_new(start: NaiveDate, end: NaiveDate) -> Result<Self> {
        if start > end {
            return Err(QuantError::InvalidCommand(format!(
                "start date {start} is after end date {end}"
            )));
        }
        Ok(Self { start, end })
    }
}

/// A single open/high/low/close/volume market bar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ohlcv {
    /// Instrument this bar describes.
    pub ticker: Ticker,

    /// Trading date of the bar.
    pub date: NaiveDate,

    /// Opening price.
    pub open: Decimal,

    /// Highest price during the period.
    pub high: Decimal,

    /// Lowest price during the period.
    pub low: Decimal,

    /// Closing price.
    pub close: Decimal,

    /// Trading volume.
    pub volume: i64,
}

impl Ohlcv {
    /// Creates a new OHLCV bar after validating price and volume invariants.
    ///
    /// # Errors
    ///
    /// Returns [`QuantError::InvalidCommand`] if any of the following hold:
    ///
    /// * `high < low`
    /// * any price (`open`, `high`, `low`, or `close`) is negative
    /// * `volume` is negative
    pub fn try_new(
        ticker: Ticker,
        date: NaiveDate,
        open: Decimal,
        high: Decimal,
        low: Decimal,
        close: Decimal,
        volume: i64,
    ) -> Result<Self> {
        if high < low {
            return Err(QuantError::InvalidCommand(format!(
                "high price {high} is below low price {low}"
            )));
        }
        for (name, value) in [
            ("open", open),
            ("high", high),
            ("low", low),
            ("close", close),
        ] {
            if value < Decimal::ZERO {
                return Err(QuantError::InvalidCommand(format!(
                    "{name} price {value} is negative"
                )));
            }
        }
        if volume < 0 {
            return Err(QuantError::InvalidCommand(format!(
                "volume {volume} is negative"
            )));
        }
        Ok(Self {
            ticker,
            date,
            open,
            high,
            low,
            close,
            volume,
        })
    }
}

/// Frequency at which market bars are aggregated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarInterval {
    /// Daily bars.
    #[default]
    Daily,

    /// Weekly bars.
    Weekly,

    /// Monthly bars.
    Monthly,
}

impl std::fmt::Display for BarInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BarInterval::Daily => write!(f, "1d"),
            BarInterval::Weekly => write!(f, "1wk"),
            BarInterval::Monthly => write!(f, "1mo"),
        }
    }
}
