//! Strategy trait definition.

use std::collections::HashMap;

use sqt_core::{Ohlcv, Result};

use crate::signal::SignalResult;

/// A trading strategy that produces signals from a price series.
///
/// Implementations must be thread-safe so they can be registered in services and
/// used across async or multi-threaded callers. The [`Debug`] bound allows
/// engines and allocations to derive `Debug` when holding a trait object.
pub trait Strategy: Send + Sync + std::fmt::Debug {
    /// Human-readable strategy name.
    fn name(&self) -> &'static str;

    /// Generate a signal for every bar in `series` using `params`.
    ///
    /// The returned vector must have the same length and order as `series`, with
    /// each [`SignalResult`] carrying the bar date and the computed signal.
    fn signals(
        &self,
        series: &[Ohlcv],
        params: &HashMap<String, String>,
    ) -> Result<Vec<SignalResult>>;
}
