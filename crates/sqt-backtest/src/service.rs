//! High-level backtest service with a registry of built-in strategies.

use std::collections::HashMap;
use std::sync::Arc;

use sqt_core::{Ohlcv, QuantError, Result};

use crate::engine::{BacktestConfig, BacktestEngine, BacktestResult};
use crate::monte_carlo::{MonteCarloResult, MonteCarloSimulator};
use crate::pair_engine::{PairBacktestConfig, PairBacktestEngine, PairSpreadStrategy};
use crate::portfolio_engine::{
    PortfolioAllocation, PortfolioBacktestEngine, PortfolioBacktestResult,
};
use crate::robustness::{RobustnessAnalyzer, RobustnessResult};
use crate::strategies::{BollingerReversion, MacdCrossover, RsiMeanReversion, SmaCrossover};
use crate::strategy::Strategy;
use crate::walk_forward::{WalkForwardConfig, WalkForwardOptimizer, WalkForwardResult};

/// High-level entry point for running backtests.
///
/// The service maintains a registry of the built-in strategies and dispatches
/// calls to the appropriate engine.
#[derive(Debug, Clone, Default)]
pub struct BacktestService {
    strategies: HashMap<String, Arc<dyn Strategy>>,
}

impl BacktestService {
    /// Creates a new service with all built-in strategies registered.
    pub fn new() -> Self {
        let mut strategies: HashMap<String, Arc<dyn Strategy>> = HashMap::new();
        strategies.insert(
            "sma_crossover".to_string(),
            Arc::new(SmaCrossover) as Arc<dyn Strategy>,
        );
        strategies.insert(
            "rsi_mean_reversion".to_string(),
            Arc::new(RsiMeanReversion) as Arc<dyn Strategy>,
        );
        strategies.insert(
            "macd_crossover".to_string(),
            Arc::new(MacdCrossover) as Arc<dyn Strategy>,
        );
        strategies.insert(
            "bollinger_reversion".to_string(),
            Arc::new(BollingerReversion) as Arc<dyn Strategy>,
        );
        Self { strategies }
    }

    /// Registers a custom strategy under `name`.
    pub fn register(&mut self, name: impl Into<String>, strategy: Arc<dyn Strategy>) {
        self.strategies.insert(name.into(), strategy);
    }

    /// Returns the names of all registered strategies.
    pub fn strategy_names(&self) -> Vec<String> {
        self.strategies.keys().cloned().collect()
    }

    fn get_strategy(&self, name: &str) -> Result<Arc<dyn Strategy>> {
        self.strategies
            .get(name)
            .cloned()
            .ok_or_else(|| QuantError::InvalidCommand(format!("unknown strategy: {name}")))
    }

    /// Runs a single named strategy over `series`.
    pub fn run_single_strategy(
        &self,
        name: &str,
        series: &[Ohlcv],
        params: HashMap<String, String>,
        config: BacktestConfig,
    ) -> Result<BacktestResult> {
        let strategy = self.get_strategy(name)?;
        let engine = BacktestEngine::new(strategy, config);
        engine.run(series, &params)
    }

    /// Runs a portfolio of allocations.
    pub fn run_portfolio(
        &self,
        allocations: Vec<PortfolioAllocation>,
        config: BacktestConfig,
    ) -> Result<PortfolioBacktestResult> {
        let engine = PortfolioBacktestEngine::new(allocations, config)?;
        engine.run()
    }

    /// Runs a pairs-trading backtest over `leg1` paired with `leg2`.
    pub fn run_pair(
        &self,
        leg1: &[Ohlcv],
        leg2: &[Ohlcv],
        config: PairBacktestConfig,
    ) -> Result<BacktestResult> {
        let strategy = Arc::new(PairSpreadStrategy::new(
            leg2.to_vec(),
            config.lookback,
            config.entry_threshold,
            config.exit_threshold,
        ));
        let engine = PairBacktestEngine::new(strategy, config);
        engine.run(leg1)
    }

    /// Runs a walk-forward optimization for the named strategy.
    pub fn run_walk_forward(
        &self,
        name: &str,
        series: &[Ohlcv],
        config: WalkForwardConfig,
    ) -> Result<WalkForwardResult> {
        let strategy = self.get_strategy(name)?;
        let optimizer = WalkForwardOptimizer::new(strategy, config);
        optimizer.run(series)
    }

    /// Runs a Monte Carlo simulation over the trades from a single strategy.
    pub fn run_monte_carlo(
        &self,
        name: &str,
        series: &[Ohlcv],
        params: HashMap<String, String>,
        config: BacktestConfig,
        simulations: Option<usize>,
        seed: Option<u64>,
    ) -> Result<MonteCarloResult> {
        let strategy = self.get_strategy(name)?;
        let engine = BacktestEngine::new(strategy, config);
        let result = engine.run(series, &params)?;
        let sim = MonteCarloSimulator::new(simulations.unwrap_or(1000), seed);
        Ok(sim.from_trades(&result.trades, config.initial_capital))
    }

    /// Runs a robustness analysis for the named strategy.
    pub fn run_robustness(
        &self,
        name: &str,
        series: &[Ohlcv],
        base_params: HashMap<String, String>,
        deltas: HashMap<String, f64>,
        config: BacktestConfig,
    ) -> Result<RobustnessResult> {
        let strategy = self.get_strategy(name)?;
        let analyzer = RobustnessAnalyzer::new(strategy, config);
        analyzer.analyze(series, &base_params, &deltas)
    }
}
