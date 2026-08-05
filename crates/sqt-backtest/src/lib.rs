//! Backtest crate for the Standard Tools Rust port.
//!
//! Provides signal generation, strategy traits, built-in strategies, trade
//! records, and a family of backtest engines: single-asset, portfolio, pairs,
//! walk-forward optimization, Monte Carlo simulation, and robustness analysis.

pub mod engine;
pub mod monte_carlo;
pub mod pair_engine;
pub mod portfolio_engine;
pub mod robustness;
pub mod service;
pub mod signal;
pub mod strategies;
pub mod strategy;
pub mod trade;
pub mod walk_forward;

mod metrics;

pub use engine::{BacktestConfig, BacktestEngine, BacktestResult};
pub use monte_carlo::{ConfidenceInterval, MonteCarloResult, MonteCarloSimulator};
pub use pair_engine::{
    PairBacktestConfig, PairBacktestEngine, PairBacktestResult, PairSpreadStrategy,
};
pub use portfolio_engine::{PortfolioAllocation, PortfolioBacktestEngine, PortfolioBacktestResult};
pub use robustness::{MetricStats, RobustnessAnalyzer, RobustnessResult};
pub use service::BacktestService;
pub use signal::{Signal, SignalResult};
pub use strategies::{BollingerReversion, MacdCrossover, RsiMeanReversion, SmaCrossover};
pub use strategy::Strategy;
pub use trade::{Trade, TradeSide};
pub use walk_forward::{
    OptimizationMetric, WalkForwardConfig, WalkForwardOptimizer, WalkForwardResult,
};
