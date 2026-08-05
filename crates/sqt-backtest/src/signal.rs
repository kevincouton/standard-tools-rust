//! Signal types used by strategies and backtest engines.

use chrono::NaiveDate;

/// A trading signal produced by a strategy for a specific bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signal {
    /// Enter or remain long.
    Buy,
    /// Enter or remain short / exit long.
    Sell,
    /// Take no action.
    Hold,
}

/// A dated signal result aligned to an [`Ohlcv`](sqt_core::Ohlcv) bar.
#[derive(Debug, Clone, PartialEq)]
pub struct SignalResult {
    /// Trading date of the bar that produced the signal.
    pub date: NaiveDate,
    /// The signal for that bar.
    pub signal: Signal,
}
