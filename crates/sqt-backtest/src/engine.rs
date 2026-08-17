//! Single-strategy backtest engine.

use std::sync::Arc;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, QuantError, Result};

use crate::metrics::compute_metrics;
use crate::signal::Signal;
use crate::strategy::Strategy;
use crate::trade::{Trade, TradeSide};

/// Configuration for a single backtest run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BacktestConfig {
    /// Starting capital.
    pub initial_capital: Decimal,
    /// Commission rate per side as a decimal fraction (e.g. `0.001` for 10 bps).
    pub commission_rate: Option<Decimal>,
    /// Number of periods per year used to annualise the Sharpe ratio.
    pub periods_per_year: u32,
    /// Annualised risk-free rate used by the Sharpe ratio.
    pub risk_free_rate: f64,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            initial_capital: Decimal::from(100_000),
            commission_rate: None,
            periods_per_year: 252,
            risk_free_rate: 0.0,
        }
    }
}

/// Result of a single-strategy backtest.
#[derive(Debug, Clone, PartialEq)]
pub struct BacktestResult {
    /// Equity curve aligned to the input series dates.
    pub equity_curve: Vec<(NaiveDate, Decimal)>,
    /// Completed round-trip trades.
    pub trades: Vec<Trade>,
    /// Total return over the backtest period as a fraction of starting equity.
    pub total_return: f64,
    /// Maximum peak-to-trough drawdown as a negative fraction of equity.
    pub max_drawdown: f64,
    /// Annualised Sharpe ratio, if computable.
    pub sharpe: Option<f64>,
    /// Number of completed round-trip trades.
    pub number_of_trades: usize,
    /// Fraction of trades with positive PnL.
    pub win_rate: f64,
}

impl BacktestResult {
    /// Final equity value, if the curve is non-empty.
    pub fn final_equity(&self) -> Option<Decimal> {
        self.equity_curve.last().map(|(_, e)| *e)
    }
}

/// Engine for running a single strategy backtest with next-bar execution.
#[derive(Debug, Clone)]
pub struct BacktestEngine {
    strategy: Arc<dyn Strategy>,
    config: BacktestConfig,
}

/// Maximum number of price bars a single backtest will process.
pub const MAX_BACKTEST_BARS: usize = 50_000;

impl BacktestEngine {
    /// Creates a new backtest engine for the supplied strategy and configuration.
    pub fn new(strategy: Arc<dyn Strategy>, config: BacktestConfig) -> Self {
        Self { strategy, config }
    }

    /// Runs the strategy over `series` using `params`.
    ///
    /// Signals generated on bar `i` are executed on the open of bar `i + 1`.
    /// Open positions are closed at the last bar's close price.
    pub fn run(
        &self,
        series: &[Ohlcv],
        params: &std::collections::HashMap<String, String>,
    ) -> Result<BacktestResult> {
        if series.is_empty() {
            return Err(QuantError::InvalidCommand(
                "backtest requires a non-empty price series".to_string(),
            ));
        }
        if series.len() > MAX_BACKTEST_BARS {
            return Err(QuantError::InvalidCommand(format!(
                "backtest series exceeds maximum of {MAX_BACKTEST_BARS} bars"
            )));
        }

        let signals = self.strategy.signals(series, params)?;
        if signals.len() != series.len() {
            return Err(QuantError::DataQuality(
                "strategy signal count does not match series length".to_string(),
            ));
        }

        let commission_rate = self.config.commission_rate.unwrap_or(Decimal::ZERO);
        let mut cash = self.config.initial_capital;
        let mut position: Option<Position> = None;
        let mut trades: Vec<Trade> = Vec::new();
        let mut equity_curve: Vec<(NaiveDate, Decimal)> = Vec::with_capacity(series.len());

        for (i, bar) in series.iter().enumerate() {
            // Execute the previous bar's signal at this bar's open.
            if i > 0 {
                let signal = signals[i - 1].signal;
                match signal {
                    Signal::Buy => {
                        if !position.as_ref().is_some_and(|p| p.side == TradeSide::Long) {
                            if let Some(p) = position.take() {
                                let (t, new_cash) =
                                    close_position(p, bar.open, commission_rate, bar.date, cash);
                                cash = new_cash;
                                trades.push(t);
                            }
                            let (p, new_cash) =
                                open_long(bar.open, cash, commission_rate, bar.date);
                            cash = new_cash;
                            position = Some(p);
                        }
                    }
                    Signal::Sell => {
                        if !position
                            .as_ref()
                            .is_some_and(|p| p.side == TradeSide::Short)
                        {
                            if let Some(p) = position.take() {
                                let (t, new_cash) =
                                    close_position(p, bar.open, commission_rate, bar.date, cash);
                                cash = new_cash;
                                trades.push(t);
                            }
                            let (p, new_cash) =
                                open_short(bar.open, cash, commission_rate, bar.date);
                            cash = new_cash;
                            position = Some(p);
                        }
                    }
                    Signal::Hold => {}
                }
            }

            // Mark to market at the close.
            let equity = match &position {
                Some(p) => cash + position_market_value(p, bar.close),
                None => cash,
            };
            equity_curve.push((bar.date, equity));
        }

        // Close any open position at the last close.
        if let Some(p) = position.take() {
            let last = series.last().expect("series is non-empty");
            let (t, new_cash) = close_position(p, last.close, commission_rate, last.date, cash);
            cash = new_cash;
            trades.push(t);
            if let Some(last_equity) = equity_curve.last_mut() {
                last_equity.1 = cash;
            }
        }

        let metrics = compute_metrics(&equity_curve, self.config);
        let number_of_trades = trades.len();
        let win_rate = if trades.is_empty() {
            0.0
        } else {
            let wins = trades.iter().filter(|t| t.pnl > Decimal::ZERO).count();
            wins as f64 / trades.len() as f64
        };

        Ok(BacktestResult {
            equity_curve,
            trades,
            total_return: metrics.total_return,
            max_drawdown: metrics.max_drawdown,
            sharpe: metrics.sharpe,
            number_of_trades,
            win_rate,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct Position {
    side: TradeSide,
    entry_price: Decimal,
    quantity: Decimal,
    entry_date: NaiveDate,
}

fn open_long(
    price: Decimal,
    cash: Decimal,
    commission_rate: Decimal,
    date: NaiveDate,
) -> (Position, Decimal) {
    let quantity = if price == Decimal::ZERO {
        Decimal::ZERO
    } else {
        // Reserve enough cash to pay commission so cash never goes negative.
        cash / (price * (Decimal::ONE + commission_rate))
    };
    let commission = quantity * price * commission_rate;
    let position = Position {
        side: TradeSide::Long,
        entry_price: price,
        quantity,
        entry_date: date,
    };
    (position, cash - quantity * price - commission)
}

fn open_short(
    price: Decimal,
    cash: Decimal,
    commission_rate: Decimal,
    date: NaiveDate,
) -> (Position, Decimal) {
    let quantity = if price == Decimal::ZERO {
        Decimal::ZERO
    } else {
        // Reserve enough cash to pay commission so cash never goes negative.
        cash / (price * (Decimal::ONE + commission_rate))
    };
    let commission = quantity * price * commission_rate;
    let position = Position {
        side: TradeSide::Short,
        entry_price: price,
        quantity,
        entry_date: date,
    };
    // Opening a short is cash-neutral apart from the commission; proceeds are
    // settled when the position is closed.
    (position, cash - commission)
}

fn close_position(
    position: Position,
    price: Decimal,
    commission_rate: Decimal,
    date: NaiveDate,
    cash: Decimal,
) -> (Trade, Decimal) {
    let commission = position.quantity * price * commission_rate;
    let entry_commission = position.quantity * position.entry_price * commission_rate;
    let (pnl, new_cash) = match position.side {
        TradeSide::Long => {
            let pnl =
                position.quantity * (price - position.entry_price) - entry_commission - commission;
            let new_cash = cash + position.quantity * price - commission;
            (pnl, new_cash)
        }
        TradeSide::Short => {
            let pnl =
                position.quantity * (position.entry_price - price) - entry_commission - commission;
            // Settle the short at exit: return the entry notional, pay the exit notional,
            // and pay the exit commission.
            let new_cash = cash + position.quantity * position.entry_price
                - position.quantity * price
                - commission;
            (pnl, new_cash)
        }
    };

    let trade = Trade {
        entry_date: position.entry_date,
        exit_date: date,
        entry_price: position.entry_price,
        exit_price: price,
        quantity: position.quantity,
        side: position.side,
        pnl,
    };
    (trade, new_cash)
}

fn position_market_value(position: &Position, price: Decimal) -> Decimal {
    match position.side {
        TradeSide::Long => position.quantity * price,
        TradeSide::Short => -(position.quantity * price),
    }
}
