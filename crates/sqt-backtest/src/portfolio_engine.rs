//! Portfolio-level backtest engine.
//!
//! # Date-range assumptions
//!
//! Each allocation is backtested independently over its own price series. The
//! portfolio engine aggregates the resulting equity curves by date, summing the
//! marked-to-market equity of all sleeves that have a value for a given date.
//! Callers should ensure that all series cover the desired portfolio date range;
//! dates missing from an allocation simply contribute nothing to the aggregate on
//! that day.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::NaiveDate;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, QuantError, Result};

use crate::engine::{BacktestConfig, BacktestEngine, BacktestResult};
use crate::metrics::compute_metrics;
use crate::strategy::Strategy;

/// An allocation within a portfolio backtest.
#[derive(Debug, Clone)]
pub struct PortfolioAllocation {
    /// Human-readable label for this sleeve, e.g. `"AAPL SMA"`.
    pub label: String,
    /// Price series for the sleeve.
    pub series: Vec<Ohlcv>,
    /// Strategy to run.
    pub strategy: Arc<dyn Strategy>,
    /// Strategy parameters.
    pub params: HashMap<String, String>,
    /// Allocation weight. Weights are normalised to sum to one internally.
    pub weight: f64,
}

/// Result of a portfolio backtest.
#[derive(Debug, Clone, PartialEq)]
pub struct PortfolioBacktestResult {
    /// Aggregated portfolio equity curve.
    pub equity_curve: Vec<(NaiveDate, Decimal)>,
    /// Individual backtest result for each labelled sleeve.
    pub per_asset: HashMap<String, BacktestResult>,
    /// Total return of the aggregated portfolio.
    pub total_return: f64,
    /// Maximum drawdown of the aggregated portfolio.
    pub max_drawdown: f64,
    /// Annualised Sharpe ratio of the aggregated portfolio, if computable.
    pub sharpe: Option<f64>,
}

/// Engine that runs multiple strategies on multiple assets and aggregates the
/// results into a single portfolio equity curve.
#[derive(Debug, Clone)]
pub struct PortfolioBacktestEngine {
    allocations: Vec<PortfolioAllocation>,
    config: BacktestConfig,
}

impl PortfolioBacktestEngine {
    /// Creates a new portfolio engine.
    ///
    /// # Errors
    ///
    /// Returns [`QuantError::InvalidCommand`] if `allocations` is empty or if all
    /// weights are zero.
    pub fn new(allocations: Vec<PortfolioAllocation>, config: BacktestConfig) -> Result<Self> {
        if allocations.is_empty() {
            return Err(QuantError::InvalidCommand(
                "portfolio backtest requires at least one allocation".to_string(),
            ));
        }
        let total_weight: f64 = allocations.iter().map(|a| a.weight).sum();
        if total_weight == 0.0 {
            return Err(QuantError::InvalidCommand(
                "portfolio allocations must have a non-zero total weight".to_string(),
            ));
        }
        Ok(Self {
            allocations,
            config,
        })
    }

    /// Runs each allocation with a capital slice proportional to its weight and
    /// aggregates the resulting equity curves by date.
    pub fn run(&self) -> Result<PortfolioBacktestResult> {
        let total_weight: f64 = self.allocations.iter().map(|a| a.weight).sum();
        let mut per_asset: HashMap<String, BacktestResult> = HashMap::new();
        let mut weighted_curves: Vec<Vec<(NaiveDate, Decimal)>> = Vec::new();

        for allocation in &self.allocations {
            let normalised_weight = allocation.weight / total_weight;
            let slice_capital = self.config.initial_capital
                * Decimal::from_f64(normalised_weight).unwrap_or(Decimal::ZERO);
            let slice_config = BacktestConfig {
                initial_capital: slice_capital,
                ..self.config
            };
            let engine = BacktestEngine::new(allocation.strategy.clone(), slice_config);
            let result = engine.run(&allocation.series, &allocation.params)?;
            weighted_curves.push(result.equity_curve.clone());
            per_asset.insert(allocation.label.clone(), result);
        }

        let equity_curve = aggregate_equity_curves(&weighted_curves)?;
        let metrics = compute_metrics(&equity_curve, self.config);

        Ok(PortfolioBacktestResult {
            equity_curve,
            per_asset,
            total_return: metrics.total_return,
            max_drawdown: metrics.max_drawdown,
            sharpe: metrics.sharpe,
        })
    }
}

fn aggregate_equity_curves(
    curves: &[Vec<(NaiveDate, Decimal)>],
) -> Result<Vec<(NaiveDate, Decimal)>> {
    if curves.is_empty() {
        return Ok(Vec::new());
    }

    // Collect all dates and sum equities.
    let mut sums: HashMap<NaiveDate, Decimal> = HashMap::new();
    for curve in curves {
        for (date, equity) in curve {
            *sums.entry(*date).or_insert(Decimal::ZERO) += *equity;
        }
    }

    let mut dates: Vec<NaiveDate> = sums.keys().copied().collect();
    dates.sort();

    Ok(dates
        .into_iter()
        .map(|d| (d, sums.get(&d).copied().unwrap_or(Decimal::ZERO)))
        .collect())
}
