//! Technical indicators crate for the Standard Tools Rust port.
//!
//! Provides a unified [`IndicatorCalculator`] entry point that dispatches to
//! implementations of common technical indicators such as SMA, EMA, RSI, MACD,
//! Bollinger Bands, ATR, OBV and VWAP. All indicator calculations operate on
//! the [`Ohlcv`] bars provided by `sqt-core` and return [`Decimal`] values
//! aligned to the input dates.

pub mod calculator;
pub mod indicators;

pub use calculator::{IndicatorCalculator, IndicatorResult};

// Re-export types used in public documentation so intra-doc links resolve.
#[doc(no_inline)]
pub use rust_decimal::Decimal;
#[doc(no_inline)]
pub use sqt_core::Ohlcv;
