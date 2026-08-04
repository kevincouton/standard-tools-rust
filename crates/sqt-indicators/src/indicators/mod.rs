//! Individual technical indicator implementations.

use rust_decimal::Decimal;
use sqt_core::{QuantError, Result};
use std::collections::HashMap;

pub mod atr;
pub mod bollinger;
pub mod ema;
pub mod macd;
pub mod obv;
pub mod rsi;
pub mod sma;
pub mod vwap;

/// Parse a `usize` parameter, falling back to a default when absent.
pub(crate) fn parse_param_usize(
    params: &HashMap<String, String>,
    key: &str,
    default: usize,
) -> Result<usize> {
    match params.get(key) {
        Some(value) => value
            .parse::<usize>()
            .map_err(|_| QuantError::InvalidCommand(format!("invalid value for {key}: {value}"))),
        None => Ok(default),
    }
}

/// Compute the arithmetic mean of a slice of decimals.
///
/// Returns `None` when the input slice is empty, avoiding division by zero.
pub(crate) fn decimal_mean(values: &[Decimal]) -> Option<Decimal> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().copied().sum::<Decimal>() / Decimal::from(values.len() as i64))
}
