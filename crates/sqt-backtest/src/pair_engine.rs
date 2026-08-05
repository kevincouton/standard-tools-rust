//! Pairs-trading backtest engine.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::NaiveDate;
use rust_decimal::prelude::ToPrimitive;
use sqt_analysis::cointegration;
use sqt_core::{Ohlcv, QuantError, Result};

use crate::engine::{BacktestConfig, BacktestResult};
use crate::signal::{Signal, SignalResult};
use crate::strategy::Strategy;
use crate::trade::TradeSide;

/// Configuration specific to pairs-trading backtests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairBacktestConfig {
    /// Standard backtest settings.
    pub backtest: BacktestConfig,
    /// Rolling window size used to estimate the cointegrating relationship.
    pub lookback: usize,
    /// Z-score threshold at which to enter the spread.
    pub entry_threshold: f64,
    /// Z-score threshold at which to exit the spread.
    pub exit_threshold: f64,
}

impl Default for PairBacktestConfig {
    fn default() -> Self {
        Self {
            backtest: BacktestConfig::default(),
            lookback: 60,
            entry_threshold: 2.0,
            exit_threshold: 0.5,
        }
    }
}

/// Result of a pairs-trading backtest.
pub type PairBacktestResult = BacktestResult;

/// A synthetic spread strategy for pairs trading.
///
/// The strategy computes a rolling cointegration relationship between two price
/// series and produces `Buy` signals when the normalised z-score falls below
/// `-entry_threshold` and `Sell` signals when it rises above `entry_threshold`.
#[derive(Debug, Clone)]
pub struct PairSpreadStrategy {
    leg2_closes: Vec<f64>,
    dates: Vec<NaiveDate>,
    lookback: usize,
    entry_threshold: f64,
    exit_threshold: f64,
}

impl PairSpreadStrategy {
    /// Creates a new spread strategy for the paired leg.
    pub fn new(
        leg2: Vec<Ohlcv>,
        lookback: usize,
        entry_threshold: f64,
        exit_threshold: f64,
    ) -> Self {
        let dates: Vec<NaiveDate> = leg2.iter().map(|bar| bar.date).collect();
        let leg2_closes: Vec<f64> = leg2
            .iter()
            .map(|bar| bar.close.to_f64().unwrap_or(0.0))
            .collect();
        Self {
            leg2_closes,
            dates,
            lookback,
            entry_threshold,
            exit_threshold,
        }
    }

    fn z_scores(&self, leg1: &[Ohlcv]) -> Result<Vec<(NaiveDate, Option<f64>)>> {
        if leg1.len() != self.dates.len() {
            return Err(QuantError::DataQuality(
                "paired series must have the same length".to_string(),
            ));
        }

        for (i, (bar, date)) in leg1.iter().zip(self.dates.iter()).enumerate() {
            if bar.date != *date {
                return Err(QuantError::DataQuality(format!(
                    "paired series date mismatch at index {i}: leg1={} leg2={}",
                    bar.date, date
                )));
            }
        }

        let closes1: Vec<f64> = leg1
            .iter()
            .map(|bar| bar.close.to_f64().unwrap_or(0.0))
            .collect();

        let mut z_scores = Vec::with_capacity(closes1.len());
        for i in 0..closes1.len() {
            if i + 1 < self.lookback {
                z_scores.push((self.dates[i], None));
                continue;
            }
            let window1: Vec<f64> = closes1[i + 1 - self.lookback..=i].to_vec();
            let window2: Vec<f64> = self.leg2_closes[i + 1 - self.lookback..=i].to_vec();

            let result = cointegration(&window1, &window2)?;
            z_scores.push((self.dates[i], Some(result.z_score)));
        }

        Ok(z_scores)
    }
}

impl Strategy for PairSpreadStrategy {
    fn name(&self) -> &'static str {
        "pair_spread"
    }

    fn signals(
        &self,
        series: &[Ohlcv],
        _params: &HashMap<String, String>,
    ) -> Result<Vec<SignalResult>> {
        let z_scores = self.z_scores(series)?;
        let mut signals = Vec::with_capacity(series.len());
        let mut position: Option<TradeSide> = None;

        for (i, bar) in series.iter().enumerate() {
            let z = z_scores.get(i).and_then(|(_, z)| *z);
            let signal = match z {
                Some(z) => {
                    if let Some(side) = position {
                        if z.abs() < self.exit_threshold {
                            position = None;
                            match side {
                                TradeSide::Long => Signal::Sell,
                                TradeSide::Short => Signal::Buy,
                            }
                        } else {
                            Signal::Hold
                        }
                    } else if z > self.entry_threshold {
                        position = Some(TradeSide::Short);
                        Signal::Sell
                    } else if z < -self.entry_threshold {
                        position = Some(TradeSide::Long);
                        Signal::Buy
                    } else {
                        Signal::Hold
                    }
                }
                None => Signal::Hold,
            };
            signals.push(SignalResult {
                date: bar.date,
                signal,
            });
        }

        Ok(signals)
    }
}

/// Engine for running a pairs-trading backtest.
#[derive(Debug, Clone)]
pub struct PairBacktestEngine {
    strategy: Arc<dyn Strategy>,
    config: PairBacktestConfig,
}

impl PairBacktestEngine {
    /// Creates a new pairs-trading engine from a spread strategy and config.
    pub fn new(strategy: Arc<dyn Strategy>, config: PairBacktestConfig) -> Self {
        Self { strategy, config }
    }

    /// Runs the pairs backtest over `leg1`.
    ///
    /// The paired leg is already baked into the [`PairSpreadStrategy`].
    pub fn run(&self, leg1: &[Ohlcv]) -> Result<PairBacktestResult> {
        use crate::engine::BacktestEngine;
        let engine = BacktestEngine::new(self.strategy.clone(), self.config.backtest);
        engine.run(leg1, &HashMap::new())
    }
}
