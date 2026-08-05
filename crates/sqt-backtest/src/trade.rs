//! Trade record type.

use chrono::NaiveDate;
use rust_decimal::Decimal;

/// Side of a backtest trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TradeSide {
    /// Long position.
    Long,
    /// Short position.
    Short,
}

/// A completed round-trip trade.
#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    /// Date the trade was entered.
    pub entry_date: NaiveDate,
    /// Date the trade was exited.
    pub exit_date: NaiveDate,
    /// Price at which the position was opened.
    pub entry_price: Decimal,
    /// Price at which the position was closed.
    pub exit_price: Decimal,
    /// Number of units traded.
    pub quantity: Decimal,
    /// Long or short.
    pub side: TradeSide,
    /// Profit/loss of the trade, after commissions.
    pub pnl: Decimal,
}
