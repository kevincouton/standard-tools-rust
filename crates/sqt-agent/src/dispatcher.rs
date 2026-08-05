//! Tool dispatcher for the `sqt-agent` crate.
//!
//! The [`ToolDispatcher`] receives [`ToolCall`] requests, validates that the
//! requested tool is registered, and routes execution to the appropriate domain
//! service. Execution errors from known tools are captured in [`ToolResult::error`]
//! so that callers receive a well-formed response; unknown tools produce a
//! [`QuantError::InvalidCommand`].

use std::collections::HashMap;
use std::sync::Arc;

use chrono::NaiveDate;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::{json, Map, Value};
use sqt_analysis::{AnalysisService, BlackScholesParams, OptionType};
use sqt_backtest::{
    BacktestConfig, BacktestService, OptimizationMetric, PairBacktestConfig, PortfolioAllocation,
    WalkForwardConfig,
};
use sqt_core::{BarInterval, DateRange, Ohlcv, QuantError, Result, Ticker};
use sqt_indicators::IndicatorCalculator;
use sqt_marketdata::MarketDataService;
use sqt_metrics::MetricsCalculator;
use sqt_portfolio::PortfolioService;
use sqt_screener::{
    Comparator, FundamentalFilter, FundamentalProvider, IndicatorFilter, ScreenerService,
};

use crate::registry;
use crate::tool::{ToolCall, ToolResult};

/// Dispatches agent tool calls to domain services.
///
/// The dispatcher is generic over the fundamental-data provider used by the
/// embedded screener. Most callers will use it with
/// [`sqt_screener::HardcodedFundamentalProvider`].
pub struct ToolDispatcher<P: FundamentalProvider> {
    market_data: Arc<MarketDataService>,
    backtest: BacktestService,
    analysis: AnalysisService,
    portfolio: PortfolioService,
    screener: ScreenerService<P>,
}

impl<P: FundamentalProvider> ToolDispatcher<P> {
    /// Creates a new dispatcher backed by the supplied services.
    pub fn new(
        market_data: Arc<MarketDataService>,
        backtest: BacktestService,
        analysis: AnalysisService,
        portfolio: PortfolioService,
        screener: ScreenerService<P>,
    ) -> Self {
        Self {
            market_data,
            backtest,
            analysis,
            portfolio,
            screener,
        }
    }

    /// Dispatches a tool call and returns its result.
    ///
    /// Unknown tools return [`QuantError::InvalidCommand`]. Errors that occur
    /// while executing a known tool are returned as a [`ToolResult`] with the
    /// `error` field populated.
    pub async fn dispatch(&self, call: ToolCall) -> Result<ToolResult> {
        if registry::find(&call.name).is_none() {
            return Err(QuantError::InvalidCommand(format!(
                "unknown tool: {}",
                call.name
            )));
        }

        match self.dispatch_known(&call).await {
            Ok(result) => Ok(result),
            Err(err) => Ok(ToolResult::err(err.to_string())),
        }
    }

    async fn dispatch_known(&self, call: &ToolCall) -> Result<ToolResult> {
        let args = &call.arguments;
        match call.name.as_str() {
            // marketdata
            "fetch_ohlcv" => {
                let ticker = parse_ticker(args, "ticker")?;
                let range = parse_date_range(args)?;
                let interval = parse_interval(args)?;
                let provider = parse_provider(args);
                let series = self
                    .market_data
                    .fetch(&ticker, range, interval, provider)
                    .await?;
                Ok(ToolResult::ok(ohlcv_series_to_value(&series)))
            }
            "fetch_multiple_ohlcv" => {
                let tickers = parse_tickers_array(args)?;
                let range = parse_date_range(args)?;
                let interval = parse_interval(args)?;
                let provider = parse_provider(args);
                let mut out = Map::new();
                for ticker in &tickers {
                    let series = self
                        .market_data
                        .fetch(ticker, range, interval, provider)
                        .await?;
                    out.insert(ticker.symbol.clone(), ohlcv_series_to_value(&series));
                }
                Ok(ToolResult::ok(Value::Object(out)))
            }

            // indicators
            "compute_sma" => self.compute_indicator("sma", args).await,
            "compute_ema" => self.compute_indicator("ema", args).await,
            "compute_rsi" => self.compute_indicator("rsi", args).await,
            "compute_macd" => self.compute_indicator("macd", args).await,
            "compute_bollinger" => self.compute_indicator("bollinger_bands", args).await,
            "compute_atr" => self.compute_indicator("atr", args).await,
            "compute_obv" => self.compute_indicator("obv", args).await,
            "compute_vwap" => self.compute_indicator("vwap", args).await,

            // metrics
            "compute_sharpe" => self.compute_metric("compute_sharpe", args).await,
            "compute_sortino" => self.compute_metric("compute_sortino", args).await,
            "compute_max_drawdown" => self.compute_metric("compute_max_drawdown", args).await,
            "compute_var" => self.compute_metric("compute_var", args).await,
            "compute_cvar" => self.compute_metric("compute_cvar", args).await,
            "compute_beta" => self.compute_metric("compute_beta", args).await,
            "compute_alpha" => self.compute_metric("compute_alpha", args).await,

            // analysis
            "linear_regression" => self.linear_regression(args).await,
            "cointegration" => self.cointegration(args).await,
            "hurst_exponent" => self.hurst_exponent(args).await,
            "pca" => self.pca(args).await,
            "correlation_matrix" => self.correlation_matrix(args).await,
            "multi_factor" => self.multi_factor(args).await,
            "black_scholes" => self.black_scholes(args),

            // backtest
            "run_sma_backtest" => self.run_single_backtest("sma_crossover", args).await,
            "run_rsi_backtest" => self.run_single_backtest("rsi_mean_reversion", args).await,
            "run_macd_backtest" => self.run_single_backtest("macd_crossover", args).await,
            "run_bollinger_backtest" => self.run_single_backtest("bollinger_reversion", args).await,
            "run_portfolio_backtest" => self.run_portfolio_backtest(args).await,
            "run_pair_backtest" => self.run_pair_backtest(args).await,
            "run_walk_forward" => self.run_walk_forward(args).await,
            "run_monte_carlo" => self.run_monte_carlo(args).await,
            "run_robustness" => self.run_robustness(args).await,

            // portfolio
            "optimize_mean_variance" => self.optimize_mean_variance(args),
            "optimize_risk_parity" => self.optimize_risk_parity(args),
            "optimize_black_litterman" => self.optimize_black_litterman(args),

            // screener
            "screen_fundamentals" => self.screen_fundamentals(args),
            "screen_with_indicators" => self.screen_with_indicators(args).await,

            // audit
            "audit_verify" | "audit_replay" => {
                Ok(ToolResult::ok(json!({ "status": "not yet implemented" })))
            }

            // utility
            "health" => Ok(ToolResult::ok(json!({ "status": "ok" }))),
            "list_tools" => Ok(ToolResult::ok(json!(registry::list()
                .iter()
                .map(|t| t.name.clone())
                .collect::<Vec<_>>()))),

            _ => Err(QuantError::InvalidCommand(format!(
                "unknown tool: {}",
                call.name
            ))),
        }
    }

    // ------------------------------------------------------------------
    // Indicators
    // ------------------------------------------------------------------

    async fn compute_indicator(&self, name: &str, args: &Value) -> Result<ToolResult> {
        let ticker = parse_ticker(args, "ticker")?;
        let range = parse_date_range(args)?;
        let interval = parse_interval(args)?;
        let provider = parse_provider(args);
        let series = self
            .market_data
            .fetch(&ticker, range, interval, provider)
            .await?;
        let params = build_indicator_params(name, args)?;
        let result = IndicatorCalculator::calculate(name, &series, &params)?;
        Ok(ToolResult::ok(indicator_to_value(&result)))
    }

    // ------------------------------------------------------------------
    // Metrics
    // ------------------------------------------------------------------

    async fn compute_metric(&self, name: &str, args: &Value) -> Result<ToolResult> {
        let risk_free_rate = get_f64(args, "risk_free_rate", 0.0)?;
        let periods_per_year = get_u32(args, "periods_per_year", 252)?;

        let result = match name {
            "compute_sharpe"
            | "compute_sortino"
            | "compute_max_drawdown"
            | "compute_var"
            | "compute_cvar" => {
                let ticker = parse_ticker(args, "ticker")?;
                let range = parse_date_range(args)?;
                let interval = parse_interval(args)?;
                let provider = parse_provider(args);
                let series = self
                    .market_data
                    .fetch(&ticker, range, interval, provider)
                    .await?;
                let returns = to_returns(&series)?;
                MetricsCalculator::from_returns(&returns, risk_free_rate, None, periods_per_year)?
            }
            "compute_beta" | "compute_alpha" => {
                let ticker = parse_ticker(args, "ticker")?;
                let benchmark = parse_ticker(args, "benchmark_ticker")?;
                let range = parse_date_range(args)?;
                let interval = parse_interval(args)?;
                let provider = parse_provider(args);
                let asset_series = self
                    .market_data
                    .fetch(&ticker, range, interval, provider)
                    .await?;
                let benchmark_series = self
                    .market_data
                    .fetch(&benchmark, range, interval, provider)
                    .await?;
                let asset_returns = to_returns(&asset_series)?;
                let benchmark_returns = to_returns(&benchmark_series)?;
                MetricsCalculator::from_returns(
                    &asset_returns,
                    risk_free_rate,
                    Some(&benchmark_returns),
                    periods_per_year,
                )?
            }
            _ => {
                return Err(QuantError::InvalidCommand(format!(
                    "unknown metric: {name}"
                )))
            }
        };

        Ok(ToolResult::ok(metrics_to_value(&result)))
    }

    // ------------------------------------------------------------------
    // Analysis
    // ------------------------------------------------------------------

    async fn linear_regression(&self, args: &Value) -> Result<ToolResult> {
        let asset = parse_ticker(args, "asset_ticker")?;
        let benchmark = parse_ticker(args, "benchmark_ticker")?;
        let range = parse_date_range(args)?;
        let interval = parse_interval(args)?;
        let provider = parse_provider(args);
        let risk_free_rate = get_f64(args, "risk_free_rate", 0.0)?;

        let asset_series = self
            .market_data
            .fetch(&asset, range, interval, provider)
            .await?;
        let benchmark_series = self
            .market_data
            .fetch(&benchmark, range, interval, provider)
            .await?;
        let result = self
            .analysis
            .regression(&asset_series, &benchmark_series, risk_free_rate)?;
        Ok(ToolResult::ok(linear_regression_to_value(&result)))
    }

    async fn cointegration(&self, args: &Value) -> Result<ToolResult> {
        let a = parse_ticker(args, "ticker_a")?;
        let b = parse_ticker(args, "ticker_b")?;
        let range = parse_date_range(args)?;
        let interval = parse_interval(args)?;
        let provider = parse_provider(args);

        let series_a = self
            .market_data
            .fetch(&a, range, interval, provider)
            .await?;
        let series_b = self
            .market_data
            .fetch(&b, range, interval, provider)
            .await?;
        let result = self.analysis.cointegration(&series_a, &series_b)?;
        Ok(ToolResult::ok(json!({
            "hedge_ratio": result.hedge_ratio,
            "half_life": result.half_life,
            "p_value": result.p_value,
            "z_score": result.z_score
        })))
    }

    async fn hurst_exponent(&self, args: &Value) -> Result<ToolResult> {
        let ticker = parse_ticker(args, "ticker")?;
        let range = parse_date_range(args)?;
        let interval = parse_interval(args)?;
        let provider = parse_provider(args);
        let max_lag = args
            .get("max_lag")
            .and_then(Value::as_u64)
            .map(|v| v as usize);

        let series = self
            .market_data
            .fetch(&ticker, range, interval, provider)
            .await?;
        let result = self.analysis.hurst(&series, max_lag)?;
        Ok(ToolResult::ok(json!({
            "exponent": result.exponent,
            "interpretation": result.interpretation
        })))
    }

    async fn pca(&self, args: &Value) -> Result<ToolResult> {
        let tickers = parse_tickers_array(args)?;
        let range = parse_date_range(args)?;
        let interval = parse_interval(args)?;
        let provider = parse_provider(args);
        let n_components = get_usize(args, "n_components", tickers.len())?;

        let mut assets: HashMap<String, Vec<Ohlcv>> = HashMap::new();
        for ticker in &tickers {
            let series = self
                .market_data
                .fetch(ticker, range, interval, provider)
                .await?;
            assets.insert(ticker.symbol.clone(), series);
        }
        let result = self.analysis.pca(&assets, n_components)?;
        Ok(ToolResult::ok(json!({
            "labels": result.labels,
            "eigenvalues": result.eigenvalues,
            "eigenvectors": result.eigenvectors,
            "explained_variance_ratio": result.explained_variance_ratio
        })))
    }

    async fn correlation_matrix(&self, args: &Value) -> Result<ToolResult> {
        let tickers = parse_tickers_array(args)?;
        let range = parse_date_range(args)?;
        let interval = parse_interval(args)?;
        let provider = parse_provider(args);

        let mut assets: HashMap<String, Vec<Ohlcv>> = HashMap::new();
        for ticker in &tickers {
            let series = self
                .market_data
                .fetch(ticker, range, interval, provider)
                .await?;
            assets.insert(ticker.symbol.clone(), series);
        }
        let result = self.analysis.correlation(&assets)?;
        Ok(ToolResult::ok(json!({
            "labels": result.labels,
            "matrix": result.matrix
        })))
    }

    async fn multi_factor(&self, args: &Value) -> Result<ToolResult> {
        let asset = parse_ticker(args, "asset_ticker")?;
        let factor_map = args
            .get("factors")
            .and_then(Value::as_object)
            .ok_or_else(|| QuantError::InvalidCommand("factors must be an object".into()))?;
        let range = parse_date_range(args)?;
        let interval = parse_interval(args)?;
        let provider = parse_provider(args);

        let asset_series = self
            .market_data
            .fetch(&asset, range, interval, provider)
            .await?;
        let mut factors: HashMap<String, Vec<Ohlcv>> = HashMap::new();
        for (name, value) in factor_map {
            let symbol = value.as_str().ok_or_else(|| {
                QuantError::InvalidCommand("factor value must be a string".into())
            })?;
            let ticker = Ticker::try_new(symbol)?;
            let series = self
                .market_data
                .fetch(&ticker, range, interval, provider)
                .await?;
            factors.insert(name.clone(), series);
        }
        let result = self.analysis.multi_factor(&asset_series, &factors)?;
        Ok(ToolResult::ok(json!({
            "factor_loadings": result.factor_loadings,
            "r_squared": result.r_squared,
            "idiosyncratic_volatility": result.idiosyncratic_volatility
        })))
    }

    fn black_scholes(&self, args: &Value) -> Result<ToolResult> {
        let spot = require_f64(args, "spot")?;
        let strike = require_f64(args, "strike")?;
        let risk_free_rate = require_f64(args, "risk_free_rate")?;
        let volatility = require_f64(args, "volatility")?;
        let time_to_maturity = require_f64(args, "time_to_maturity")?;
        let option_type = match get_str(args, "option_type")? {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            _ => {
                return Err(QuantError::InvalidCommand(
                    "option_type must be call or put".into(),
                ))
            }
        };
        let result = self.analysis.black_scholes(BlackScholesParams {
            spot,
            strike,
            risk_free_rate,
            volatility,
            time_to_maturity,
            option_type,
        })?;
        Ok(ToolResult::ok(json!({
            "price": result.price,
            "delta": result.delta,
            "gamma": result.gamma,
            "vega": result.vega,
            "theta": result.theta,
            "rho": result.rho
        })))
    }

    // ------------------------------------------------------------------
    // Backtest
    // ------------------------------------------------------------------

    async fn run_single_backtest(&self, strategy_name: &str, args: &Value) -> Result<ToolResult> {
        let ticker = parse_ticker(args, "ticker")?;
        let range = parse_date_range(args)?;
        let interval = parse_interval(args)?;
        let provider = parse_provider(args);
        let series = self
            .market_data
            .fetch(&ticker, range, interval, provider)
            .await?;
        let params = build_strategy_params(strategy_name, args)?;
        let config = parse_backtest_config(args)?;
        let result = self
            .backtest
            .run_single_strategy(strategy_name, &series, params, config)?;
        Ok(ToolResult::ok(backtest_to_value(&result)))
    }

    async fn run_portfolio_backtest(&self, args: &Value) -> Result<ToolResult> {
        let allocations_arg = args
            .get("allocations")
            .and_then(Value::as_array)
            .ok_or_else(|| QuantError::InvalidCommand("allocations must be an array".into()))?;
        let range = parse_date_range(args)?;
        let interval = parse_interval(args)?;
        let provider = parse_provider(args);
        let config = parse_backtest_config(args)?;

        let mut allocations: Vec<PortfolioAllocation> = Vec::new();
        for item in allocations_arg {
            let ticker = parse_ticker(item, "ticker")?;
            let strategy_name = get_str(item, "strategy")?;
            let weight = get_f64(item, "weight", 1.0)?;
            let params = item
                .get("params")
                .map(parse_string_map)
                .unwrap_or_else(|| Ok(HashMap::new()))?;
            let series = self
                .market_data
                .fetch(&ticker, range, interval, provider)
                .await?;
            allocations.push(PortfolioAllocation {
                label: ticker.symbol.clone(),
                series,
                strategy: strategy_by_name(strategy_name)?,
                params,
                weight,
            });
        }

        let result = self.backtest.run_portfolio(allocations, config)?;
        Ok(ToolResult::ok(portfolio_backtest_to_value(&result)))
    }

    async fn run_pair_backtest(&self, args: &Value) -> Result<ToolResult> {
        let leg1 = parse_ticker(args, "leg1_ticker")?;
        let leg2 = parse_ticker(args, "leg2_ticker")?;
        let range = parse_date_range(args)?;
        let interval = parse_interval(args)?;
        let provider = parse_provider(args);

        let leg1_series = self
            .market_data
            .fetch(&leg1, range, interval, provider)
            .await?;
        let leg2_series = self
            .market_data
            .fetch(&leg2, range, interval, provider)
            .await?;
        let config = PairBacktestConfig {
            backtest: parse_backtest_config(args)?,
            lookback: get_usize(args, "lookback", 60)?,
            entry_threshold: get_f64(args, "entry_threshold", 2.0)?,
            exit_threshold: get_f64(args, "exit_threshold", 0.5)?,
        };
        let result = self.backtest.run_pair(&leg1_series, &leg2_series, config)?;
        Ok(ToolResult::ok(backtest_to_value(&result)))
    }

    async fn run_walk_forward(&self, args: &Value) -> Result<ToolResult> {
        let ticker = parse_ticker(args, "ticker")?;
        let strategy_name = get_str(args, "strategy")?;
        let range = parse_date_range(args)?;
        let interval = parse_interval(args)?;
        let provider = parse_provider(args);
        let series = self
            .market_data
            .fetch(&ticker, range, interval, provider)
            .await?;
        let config = WalkForwardConfig {
            train_size: get_usize(args, "train_size", 0)?,
            test_size: get_usize(args, "test_size", 0)?,
            param_grid: parse_param_grid(
                args.get("param_grid")
                    .ok_or_else(|| QuantError::InvalidCommand("missing param_grid".into()))?,
            )?,
            metric: match args.get("metric").and_then(Value::as_str) {
                Some("sharpe") => OptimizationMetric::Sharpe,
                Some("win_rate") => OptimizationMetric::WinRate,
                _ => OptimizationMetric::TotalReturn,
            },
            backtest: parse_backtest_config(args)?,
        };
        let result = self
            .backtest
            .run_walk_forward(strategy_name, &series, config)?;
        Ok(ToolResult::ok(json!({
            "total_return": result.total_return,
            "max_drawdown": result.max_drawdown,
            "sharpe": result.sharpe,
            "number_of_trades": result.number_of_trades,
            "win_rate": result.win_rate,
            "selected_params": result.selected_params.iter().map(|(d, p)| json!({
                "date": d.to_string(),
                "params": p
            })).collect::<Vec<_>>()
        })))
    }

    async fn run_monte_carlo(&self, args: &Value) -> Result<ToolResult> {
        let ticker = parse_ticker(args, "ticker")?;
        let strategy_name = get_str(args, "strategy")?;
        let range = parse_date_range(args)?;
        let interval = parse_interval(args)?;
        let provider = parse_provider(args);
        let series = self
            .market_data
            .fetch(&ticker, range, interval, provider)
            .await?;
        let params = args
            .get("params")
            .map(parse_string_map)
            .unwrap_or_else(|| Ok(HashMap::new()))?;
        let config = parse_backtest_config(args)?;
        let simulations = args
            .get("simulations")
            .and_then(Value::as_u64)
            .map(|v| v as usize);
        let seed = args.get("seed").and_then(Value::as_u64);
        let result = self.backtest.run_monte_carlo(
            strategy_name,
            &series,
            params,
            config,
            simulations,
            seed,
        )?;
        Ok(ToolResult::ok(json!({
            "simulations": result.simulations,
            "final_equity_ci": {
                "lower": result.final_equity_ci.lower,
                "upper": result.final_equity_ci.upper
            },
            "max_drawdown_ci": {
                "lower": result.max_drawdown_ci.lower,
                "upper": result.max_drawdown_ci.upper
            }
        })))
    }

    async fn run_robustness(&self, args: &Value) -> Result<ToolResult> {
        let ticker = parse_ticker(args, "ticker")?;
        let strategy_name = get_str(args, "strategy")?;
        let range = parse_date_range(args)?;
        let interval = parse_interval(args)?;
        let provider = parse_provider(args);
        let series = self
            .market_data
            .fetch(&ticker, range, interval, provider)
            .await?;
        let base_params = parse_string_map(
            args.get("base_params")
                .ok_or_else(|| QuantError::InvalidCommand("missing base_params".into()))?,
        )?;
        let deltas = parse_f64_map(
            args.get("deltas")
                .ok_or_else(|| QuantError::InvalidCommand("missing deltas".into()))?,
        )?;
        let config = parse_backtest_config(args)?;
        let result =
            self.backtest
                .run_robustness(strategy_name, &series, base_params, deltas, config)?;
        Ok(ToolResult::ok(json!({
            "base": backtest_to_value(&result.base),
            "perturbation_count": result.perturbations.len(),
            "stats": {
                "mean_total_return": result.stats.mean_total_return,
                "std_total_return": result.stats.std_total_return,
                "mean_max_drawdown": result.stats.mean_max_drawdown,
                "std_max_drawdown": result.stats.std_max_drawdown,
                "mean_sharpe": result.stats.mean_sharpe,
                "std_sharpe": result.stats.std_sharpe,
                "mean_win_rate": result.stats.mean_win_rate,
                "std_win_rate": result.stats.std_win_rate
            }
        })))
    }

    // ------------------------------------------------------------------
    // Portfolio
    // ------------------------------------------------------------------

    fn optimize_mean_variance(&self, args: &Value) -> Result<ToolResult> {
        let returns = parse_returns_map(args)?;
        let risk_free_rate = get_f64(args, "risk_free_rate", 0.0)?;
        let target_return = args.get("target_return").and_then(Value::as_f64);
        let result = self
            .portfolio
            .mean_variance(&returns, risk_free_rate, target_return)?;
        Ok(ToolResult::ok(to_tool_value(result)?))
    }

    fn optimize_risk_parity(&self, args: &Value) -> Result<ToolResult> {
        let returns = parse_returns_map(args)?;
        let result = self.portfolio.risk_parity(&returns)?;
        Ok(ToolResult::ok(to_tool_value(result)?))
    }

    fn optimize_black_litterman(&self, args: &Value) -> Result<ToolResult> {
        let returns = parse_returns_map(args)?;
        let market_caps = parse_f64_map(
            args.get("market_caps")
                .ok_or_else(|| QuantError::InvalidCommand("missing market_caps".into()))?,
        )?;
        let views = parse_f64_map(
            args.get("views")
                .ok_or_else(|| QuantError::InvalidCommand("missing views".into()))?,
        )?;
        let tau = require_f64(args, "tau")?;
        let risk_aversion = require_f64(args, "risk_aversion")?;
        let result =
            self.portfolio
                .black_litterman(&returns, &market_caps, &views, tau, risk_aversion)?;
        Ok(ToolResult::ok(to_tool_value(result)?))
    }

    // ------------------------------------------------------------------
    // Screener
    // ------------------------------------------------------------------

    fn screen_fundamentals(&self, args: &Value) -> Result<ToolResult> {
        let filters = parse_fundamental_filters(args)?;
        let results = self.screener.screen(&filters)?;
        Ok(ToolResult::ok(to_tool_value(results)?))
    }

    async fn screen_with_indicators(&self, args: &Value) -> Result<ToolResult> {
        let filters = parse_fundamental_filters(args)?;
        let indicator_filters = parse_indicator_filters(args)?;
        let range = parse_date_range(args)?;
        let interval = parse_interval(args)?;
        let provider = parse_provider(args);

        let fundamental_matches = self.screener.screen(&filters)?;
        let mut ohlcv_data: HashMap<String, Vec<Ohlcv>> = HashMap::new();

        for data in &fundamental_matches {
            let ticker = Ticker::new(&data.ticker);
            let series = self
                .market_data
                .fetch(&ticker, range, interval, provider)
                .await?;
            ohlcv_data.insert(data.ticker.clone(), series);
        }

        let results =
            self.screener
                .screen_with_indicators(&filters, &indicator_filters, &ohlcv_data)?;
        Ok(ToolResult::ok(to_tool_value(results)?))
    }
}

// ----------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------

fn get_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| QuantError::InvalidCommand(format!("missing or invalid `{key}`")))
}

fn get_string(args: &Value, key: &str) -> Result<String> {
    Ok(get_str(args, key)?.to_string())
}

fn get_f64(args: &Value, key: &str, default: f64) -> Result<f64> {
    match args.get(key) {
        Some(v) => v
            .as_f64()
            .ok_or_else(|| QuantError::InvalidCommand(format!("`{key}` must be a number"))),
        None => Ok(default),
    }
}

fn require_f64(args: &Value, key: &str) -> Result<f64> {
    args.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| QuantError::InvalidCommand(format!("missing or invalid `{key}`")))
}

fn get_u32(args: &Value, key: &str, default: u32) -> Result<u32> {
    match args.get(key) {
        Some(v) => v.as_u64().map(|u| u as u32).ok_or_else(|| {
            QuantError::InvalidCommand(format!("`{key}` must be a non-negative integer"))
        }),
        None => Ok(default),
    }
}

fn get_usize(args: &Value, key: &str, default: usize) -> Result<usize> {
    match args.get(key) {
        Some(v) => v.as_u64().map(|u| u as usize).ok_or_else(|| {
            QuantError::InvalidCommand(format!("`{key}` must be a non-negative integer"))
        }),
        None => Ok(default),
    }
}

fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| QuantError::InvalidCommand(format!("invalid date `{s}`: {e}")))
}

fn parse_date_range(args: &Value) -> Result<DateRange> {
    let start = parse_date(get_str(args, "start")?)?;
    let end = parse_date(get_str(args, "end")?)?;
    DateRange::try_new(start, end)
}

fn parse_interval(args: &Value) -> Result<BarInterval> {
    match args.get("interval").and_then(Value::as_str) {
        Some("daily") | None => Ok(BarInterval::Daily),
        Some("weekly") => Ok(BarInterval::Weekly),
        Some("monthly") => Ok(BarInterval::Monthly),
        Some(other) => Err(QuantError::InvalidCommand(format!(
            "invalid interval `{other}`; must be daily, weekly, or monthly"
        ))),
    }
}

fn parse_provider(args: &Value) -> Option<&str> {
    args.get("provider").and_then(Value::as_str)
}

fn parse_ticker(args: &Value, key: &str) -> Result<Ticker> {
    Ticker::try_new(get_str(args, key)?)
}

fn parse_tickers_array(args: &Value) -> Result<Vec<Ticker>> {
    let arr = args
        .get("tickers")
        .and_then(Value::as_array)
        .ok_or_else(|| QuantError::InvalidCommand("tickers must be an array".into()))?;
    arr.iter()
        .map(|v| {
            v.as_str().map(Ticker::try_new).unwrap_or_else(|| {
                Err(QuantError::InvalidCommand("ticker must be a string".into()))
            })
        })
        .collect()
}

fn to_returns(series: &[Ohlcv]) -> Result<Vec<f64>> {
    if series.len() < 2 {
        return Err(QuantError::DataQuality(
            "need at least two bars to compute returns".into(),
        ));
    }
    let mut sorted = series.to_vec();
    sorted.sort_by_key(|b| b.date);
    sorted
        .windows(2)
        .map(|w| {
            let prev = w[0]
                .close
                .to_f64()
                .ok_or_else(|| QuantError::DataQuality("invalid previous close".into()))?;
            let curr = w[1]
                .close
                .to_f64()
                .ok_or_else(|| QuantError::DataQuality("invalid current close".into()))?;
            if prev == 0.0 {
                return Err(QuantError::DataQuality(
                    "previous close is zero; cannot compute return".into(),
                ));
            }
            Ok(curr / prev - 1.0)
        })
        .collect::<Result<Vec<_>>>()
}

/// Converts a JSON value into a string suitable for indicator/strategy parameters.
///
/// Strings are returned as-is; numbers and booleans are converted to their
/// textual representation. Arrays, objects, and null are rejected.
fn value_to_param_string(v: &Value) -> Result<String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        _ => Err(QuantError::InvalidCommand(format!(
            "parameter value must be a string, number, or boolean, got {v}"
        ))),
    }
}

fn parse_string_map(value: &Value) -> Result<HashMap<String, String>> {
    let obj = value
        .as_object()
        .ok_or_else(|| QuantError::InvalidCommand("expected object".into()))?;
    let mut map = HashMap::new();
    for (k, v) in obj {
        map.insert(k.clone(), value_to_param_string(v)?);
    }
    Ok(map)
}

fn parse_f64_map(value: &Value) -> Result<HashMap<String, f64>> {
    let obj = value
        .as_object()
        .ok_or_else(|| QuantError::InvalidCommand("expected object of numbers".into()))?;
    let mut map = HashMap::new();
    for (k, v) in obj {
        map.insert(
            k.clone(),
            v.as_f64()
                .ok_or_else(|| QuantError::InvalidCommand(format!("`{k}` must be a number")))?,
        );
    }
    Ok(map)
}

fn parse_returns_map(args: &Value) -> Result<HashMap<String, Vec<f64>>> {
    let obj = args
        .get("returns")
        .and_then(Value::as_object)
        .ok_or_else(|| QuantError::InvalidCommand("returns must be an object".into()))?;
    let mut map = HashMap::new();
    for (k, v) in obj {
        let arr = v.as_array().ok_or_else(|| {
            QuantError::InvalidCommand(format!("returns for `{k}` must be an array"))
        })?;
        let values: Result<Vec<f64>> = arr
            .iter()
            .map(|x| {
                x.as_f64()
                    .ok_or_else(|| QuantError::InvalidCommand("returns must be numbers".into()))
            })
            .collect();
        map.insert(k.clone(), values?);
    }
    Ok(map)
}

fn parse_param_grid(value: &Value) -> Result<HashMap<String, Vec<String>>> {
    let obj = value
        .as_object()
        .ok_or_else(|| QuantError::InvalidCommand("param_grid must be an object".into()))?;
    let mut map = HashMap::new();
    for (k, v) in obj {
        let arr = v
            .as_array()
            .ok_or_else(|| QuantError::InvalidCommand(format!("`{k}` must be an array")))?;
        let values: Result<Vec<String>> = arr.iter().map(value_to_param_string).collect();
        map.insert(k.clone(), values?);
    }
    Ok(map)
}

fn parse_backtest_config(args: &Value) -> Result<BacktestConfig> {
    let initial_capital = get_f64(args, "initial_capital", 100_000.0)?;
    let commission_rate = get_f64(args, "commission_rate", 0.0)?;
    let periods_per_year = get_u32(args, "periods_per_year", 252)?;
    let risk_free_rate = get_f64(args, "risk_free_rate", 0.0)?;

    Ok(BacktestConfig {
        initial_capital: Decimal::from_f64(initial_capital)
            .ok_or_else(|| QuantError::InvalidCommand("invalid initial_capital".into()))?,
        commission_rate: Some(
            Decimal::from_f64(commission_rate)
                .ok_or_else(|| QuantError::InvalidCommand("invalid commission_rate".into()))?,
        ),
        periods_per_year,
        risk_free_rate,
    })
}

fn build_indicator_params(name: &str, args: &Value) -> Result<HashMap<String, String>> {
    let mut params = HashMap::new();
    match name {
        "sma" | "ema" | "rsi" | "atr" => {
            if let Some(v) = args.get("period").and_then(Value::as_u64) {
                params.insert("period".to_string(), v.to_string());
            }
        }
        "macd" => {
            if let Some(v) = args.get("fast").and_then(Value::as_u64) {
                params.insert("fast".to_string(), v.to_string());
            }
            if let Some(v) = args.get("slow").and_then(Value::as_u64) {
                params.insert("slow".to_string(), v.to_string());
            }
            if let Some(v) = args.get("signal").and_then(Value::as_u64) {
                params.insert("signal".to_string(), v.to_string());
            }
        }
        "bollinger_bands" => {
            if let Some(v) = args.get("period").and_then(Value::as_u64) {
                params.insert("period".to_string(), v.to_string());
            }
            if let Some(v) = args.get("std_dev").and_then(Value::as_u64) {
                params.insert("std_dev".to_string(), v.to_string());
            }
        }
        _ => {}
    }
    Ok(params)
}

fn build_strategy_params(strategy_name: &str, args: &Value) -> Result<HashMap<String, String>> {
    let mut params = HashMap::new();
    match strategy_name {
        "sma_crossover" => {
            if let Some(v) = args.get("fast").and_then(Value::as_u64) {
                params.insert("fast".to_string(), v.to_string());
            }
            if let Some(v) = args.get("slow").and_then(Value::as_u64) {
                params.insert("slow".to_string(), v.to_string());
            }
        }
        "rsi_mean_reversion" => {
            if let Some(v) = args.get("period").and_then(Value::as_u64) {
                params.insert("period".to_string(), v.to_string());
            }
            if let Some(v) = args.get("oversold") {
                params.insert("oversold".to_string(), value_to_param_string(v)?);
            }
            if let Some(v) = args.get("overbought") {
                params.insert("overbought".to_string(), value_to_param_string(v)?);
            }
        }
        "macd_crossover" => {
            if let Some(v) = args.get("fast").and_then(Value::as_u64) {
                params.insert("fast".to_string(), v.to_string());
            }
            if let Some(v) = args.get("slow").and_then(Value::as_u64) {
                params.insert("slow".to_string(), v.to_string());
            }
            if let Some(v) = args.get("signal").and_then(Value::as_u64) {
                params.insert("signal".to_string(), v.to_string());
            }
        }
        "bollinger_reversion" => {
            if let Some(v) = args.get("period").and_then(Value::as_u64) {
                params.insert("period".to_string(), v.to_string());
            }
            if let Some(v) = args.get("std_dev").and_then(Value::as_u64) {
                params.insert("std_dev".to_string(), v.to_string());
            }
        }
        _ => {}
    }
    Ok(params)
}

fn strategy_by_name(name: &str) -> Result<Arc<dyn sqt_backtest::Strategy>> {
    match name {
        "sma_crossover" => Ok(Arc::new(sqt_backtest::SmaCrossover)),
        "rsi_mean_reversion" => Ok(Arc::new(sqt_backtest::RsiMeanReversion)),
        "macd_crossover" => Ok(Arc::new(sqt_backtest::MacdCrossover)),
        "bollinger_reversion" => Ok(Arc::new(sqt_backtest::BollingerReversion)),
        _ => Err(QuantError::InvalidCommand(format!(
            "unknown strategy: {name}"
        ))),
    }
}

fn parse_fundamental_filters(args: &Value) -> Result<Vec<FundamentalFilter>> {
    let arr = args
        .get("filters")
        .and_then(Value::as_array)
        .ok_or_else(|| QuantError::InvalidCommand("filters must be an array".into()))?;
    let mut filters = Vec::new();
    for item in arr {
        let field = get_str(item, "field")?;
        let comparator = get_str(item, "comparator")?;
        let value = require_f64(item, "value")?;
        let filter = match (field, comparator) {
            ("pe", "lt") => FundamentalFilter::PeLt(value),
            ("pe", "gt") => FundamentalFilter::PeGt(value),
            ("market_cap", "lt") => FundamentalFilter::MarketCapLt(value),
            ("market_cap", "gt") => FundamentalFilter::MarketCapGt(value),
            ("pb", "lt") => FundamentalFilter::PbLt(value),
            ("pb", "gt") => FundamentalFilter::PbGt(value),
            ("dividend_yield", "lt") => FundamentalFilter::DividendYieldLt(value),
            ("dividend_yield", "gt") => FundamentalFilter::DividendYieldGt(value),
            ("eps_growth", "lt") => FundamentalFilter::EpsGrowthLt(value),
            ("eps_growth", "gt") => FundamentalFilter::EpsGrowthGt(value),
            ("debt_to_equity", "lt") => FundamentalFilter::DebtToEquityLt(value),
            ("debt_to_equity", "gt") => FundamentalFilter::DebtToEquityGt(value),
            ("roe", "lt") => FundamentalFilter::RoeLt(value),
            ("roe", "gt") => FundamentalFilter::RoeGt(value),
            _ => {
                return Err(QuantError::InvalidCommand(format!(
                    "unknown filter {field} {comparator}"
                )))
            }
        };
        filters.push(filter);
    }
    Ok(filters)
}

fn parse_indicator_filters(args: &Value) -> Result<Vec<IndicatorFilter>> {
    let arr = args
        .get("indicator_filters")
        .and_then(Value::as_array)
        .ok_or_else(|| QuantError::InvalidCommand("indicator_filters must be an array".into()))?;
    let mut filters = Vec::new();
    for item in arr {
        let indicator = get_string(item, "indicator")?;
        let comparator = match get_str(item, "comparator")? {
            "lt" => Comparator::Lt,
            "gt" => Comparator::Gt,
            _ => {
                return Err(QuantError::InvalidCommand(
                    "comparator must be lt or gt".into(),
                ))
            }
        };
        let threshold = require_f64(item, "threshold")?;
        let params = item
            .get("params")
            .map(parse_string_map)
            .unwrap_or_else(|| Ok(HashMap::new()))?;
        filters.push(IndicatorFilter {
            indicator,
            params,
            comparator,
            threshold,
        });
    }
    Ok(filters)
}

fn ohlcv_series_to_value(series: &[Ohlcv]) -> Value {
    Value::Array(series.iter().map(ohlcv_to_value).collect())
}

fn ohlcv_to_value(bar: &Ohlcv) -> Value {
    json!({
        "date": bar.date.to_string(),
        "open": bar.open.to_f64(),
        "high": bar.high.to_f64(),
        "low": bar.low.to_f64(),
        "close": bar.close.to_f64(),
        "volume": bar.volume
    })
}

fn indicator_to_value(result: &sqt_indicators::IndicatorResult) -> Value {
    let values = result
        .values
        .iter()
        .map(|(date, v)| json!({ "date": date.to_string(), "value": v.and_then(|d| d.to_f64()) }))
        .collect::<Vec<_>>();
    let extra_series: Map<String, Value> = result
        .extra_series
        .iter()
        .map(|(name, series)| {
            let arr = series
                .iter()
                .map(|(date, v)| {
                    json!({ "date": date.to_string(), "value": v.and_then(|d| d.to_f64()) })
                })
                .collect::<Vec<_>>();
            (name.clone(), Value::Array(arr))
        })
        .collect();
    json!({
        "name": result.name,
        "params": result.params,
        "values": values,
        "extra_series": extra_series
    })
}

fn metrics_to_value(result: &sqt_metrics::MetricsResult) -> Value {
    json!({
        "sharpe": result.sharpe,
        "sortino": result.sortino,
        "max_drawdown": result.max_drawdown,
        "var": result.var,
        "cvar": result.cvar,
        "beta": result.beta,
        "alpha": result.alpha,
        "annualized_return": result.annualized_return,
        "volatility": result.volatility,
        "total_return": result.total_return
    })
}

fn linear_regression_to_value(result: &sqt_analysis::LinearRegressionResult) -> Value {
    json!({
        "alpha": result.alpha,
        "beta": result.beta,
        "r_squared": result.r_squared,
        "p_value": result.p_value,
        "residuals": result.residuals
    })
}

fn backtest_to_value(result: &sqt_backtest::BacktestResult) -> Value {
    let equity_curve = result
        .equity_curve
        .iter()
        .map(|(date, equity)| json!({ "date": date.to_string(), "equity": equity.to_f64() }))
        .collect::<Vec<_>>();
    let trades = result
        .trades
        .iter()
        .map(|t| {
            json!({
                "entry_date": t.entry_date.to_string(),
                "exit_date": t.exit_date.to_string(),
                "entry_price": t.entry_price.to_f64(),
                "exit_price": t.exit_price.to_f64(),
                "quantity": t.quantity.to_f64(),
                "side": match t.side {
                    sqt_backtest::TradeSide::Long => "long",
                    sqt_backtest::TradeSide::Short => "short",
                },
                "pnl": t.pnl.to_f64()
            })
        })
        .collect::<Vec<_>>();
    json!({
        "total_return": result.total_return,
        "max_drawdown": result.max_drawdown,
        "sharpe": result.sharpe,
        "number_of_trades": result.number_of_trades,
        "win_rate": result.win_rate,
        "equity_curve": equity_curve,
        "trades": trades
    })
}

fn to_tool_value<T: Serialize>(value: T) -> Result<Value> {
    serde_json::to_value(value)
        .map_err(|e| QuantError::Internal(anyhow::anyhow!("serialization failed: {e}")))
}

fn portfolio_backtest_to_value(result: &sqt_backtest::PortfolioBacktestResult) -> Value {
    let equity_curve = result
        .equity_curve
        .iter()
        .map(|(date, equity)| json!({ "date": date.to_string(), "equity": equity.to_f64() }))
        .collect::<Vec<_>>();
    let per_asset: Map<String, Value> = result
        .per_asset
        .iter()
        .map(|(label, res)| (label.clone(), backtest_to_value(res)))
        .collect();
    json!({
        "total_return": result.total_return,
        "max_drawdown": result.max_drawdown,
        "sharpe": result.sharpe,
        "equity_curve": equity_curve,
        "per_asset": per_asset
    })
}
