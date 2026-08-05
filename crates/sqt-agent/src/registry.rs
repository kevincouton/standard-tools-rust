//! Tool registry for the `sqt-agent` crate.
//!
//! The registry exposes a static list of at least 42 tool definitions across
//! market data, indicators, metrics, analysis, backtesting, portfolio
//! construction, screening, audit, and utility categories. Each definition
//! carries a JSON Schema describing its parameters.

use std::sync::LazyLock;

use serde_json::json;

use crate::tool::ToolDefinition;

/// Static list of all registered tool definitions.
static TOOLS: LazyLock<Vec<ToolDefinition>> = LazyLock::new(|| {
    vec![
        // marketdata
        ToolDefinition {
            name: "fetch_ohlcv".to_string(),
            description: "Fetch OHLCV bars for a single ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "fetch_multiple_ohlcv".to_string(),
            description: "Fetch OHLCV bars for multiple tickers.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tickers": { "type": "array", "items": { "type": "string" } },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" }
                },
                "required": ["tickers", "start", "end"]
            }),
        },
        // indicators
        ToolDefinition {
            name: "compute_sma".to_string(),
            description: "Compute the Simple Moving Average for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "period": { "type": "integer", "minimum": 1 }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "compute_ema".to_string(),
            description: "Compute the Exponential Moving Average for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "period": { "type": "integer", "minimum": 1 }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "compute_rsi".to_string(),
            description: "Compute the Relative Strength Index for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "period": { "type": "integer", "minimum": 1 }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "compute_macd".to_string(),
            description: "Compute MACD for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "fast": { "type": "integer", "minimum": 1 },
                    "slow": { "type": "integer", "minimum": 1 },
                    "signal": { "type": "integer", "minimum": 1 }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "compute_bollinger".to_string(),
            description: "Compute Bollinger Bands for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "period": { "type": "integer", "minimum": 1 },
                    "std_dev": { "type": "integer", "minimum": 1 }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "compute_atr".to_string(),
            description: "Compute the Average True Range for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "period": { "type": "integer", "minimum": 1 }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "compute_obv".to_string(),
            description: "Compute On-Balance Volume for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "compute_vwap".to_string(),
            description: "Compute the Volume Weighted Average Price for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        // metrics
        ToolDefinition {
            name: "compute_sharpe".to_string(),
            description: "Compute the annualised Sharpe ratio for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "risk_free_rate": { "type": "number" },
                    "periods_per_year": { "type": "integer", "minimum": 1 }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "compute_sortino".to_string(),
            description: "Compute the annualised Sortino ratio for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "risk_free_rate": { "type": "number" },
                    "periods_per_year": { "type": "integer", "minimum": 1 }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "compute_max_drawdown".to_string(),
            description: "Compute the maximum drawdown for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "compute_var".to_string(),
            description: "Compute historical Value at Risk for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "compute_cvar".to_string(),
            description: "Compute historical Conditional Value at Risk for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "compute_beta".to_string(),
            description: "Compute beta of a ticker against a benchmark.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "benchmark_ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "risk_free_rate": { "type": "number" },
                    "periods_per_year": { "type": "integer", "minimum": 1 }
                },
                "required": ["ticker", "benchmark_ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "compute_alpha".to_string(),
            description: "Compute annualised Jensen's alpha for a ticker against a benchmark."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "benchmark_ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "risk_free_rate": { "type": "number" },
                    "periods_per_year": { "type": "integer", "minimum": 1 }
                },
                "required": ["ticker", "benchmark_ticker", "start", "end"]
            }),
        },
        // analysis
        ToolDefinition {
            name: "linear_regression".to_string(),
            description: "Run a single-variable linear regression of an asset on a benchmark."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "asset_ticker": { "type": "string" },
                    "benchmark_ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "risk_free_rate": { "type": "number" }
                },
                "required": ["asset_ticker", "benchmark_ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "cointegration".to_string(),
            description: "Test for cointegration between two price series.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker_a": { "type": "string" },
                    "ticker_b": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" }
                },
                "required": ["ticker_a", "ticker_b", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "hurst_exponent".to_string(),
            description: "Estimate the Hurst exponent of a price series.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "max_lag": { "type": "integer", "minimum": 1 }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "pca".to_string(),
            description: "Run PCA on the aligned return series of multiple tickers.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tickers": { "type": "array", "items": { "type": "string" } },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "n_components": { "type": "integer", "minimum": 1 }
                },
                "required": ["tickers", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "correlation_matrix".to_string(),
            description: "Compute a Pearson correlation matrix for multiple tickers.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tickers": { "type": "array", "items": { "type": "string" } },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" }
                },
                "required": ["tickers", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "multi_factor".to_string(),
            description: "Run a multi-factor regression of an asset on supplied factors."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "asset_ticker": { "type": "string" },
                    "factors": { "type": "object", "additionalProperties": { "type": "string" } },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" }
                },
                "required": ["asset_ticker", "factors", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "black_scholes".to_string(),
            description: "Price a European option with the Black-Scholes formula.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "spot": { "type": "number", "minimum": 0.0 },
                    "strike": { "type": "number", "minimum": 0.0 },
                    "risk_free_rate": { "type": "number" },
                    "volatility": { "type": "number", "minimum": 0.0 },
                    "time_to_maturity": { "type": "number", "minimum": 0.0 },
                    "option_type": { "type": "string", "enum": ["call", "put"] }
                },
                "required": ["spot", "strike", "risk_free_rate", "volatility", "time_to_maturity", "option_type"]
            }),
        },
        // backtest
        ToolDefinition {
            name: "run_sma_backtest".to_string(),
            description: "Run an SMA crossover backtest for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "fast": { "type": "integer", "minimum": 1 },
                    "slow": { "type": "integer", "minimum": 1 },
                    "initial_capital": { "type": "number", "minimum": 0.0 },
                    "commission_rate": { "type": "number", "minimum": 0.0 },
                    "periods_per_year": { "type": "integer", "minimum": 1 },
                    "risk_free_rate": { "type": "number" }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "run_rsi_backtest".to_string(),
            description: "Run an RSI mean-reversion backtest for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "period": { "type": "integer", "minimum": 1 },
                    "oversold": { "type": "number" },
                    "overbought": { "type": "number" },
                    "initial_capital": { "type": "number", "minimum": 0.0 },
                    "commission_rate": { "type": "number", "minimum": 0.0 },
                    "periods_per_year": { "type": "integer", "minimum": 1 },
                    "risk_free_rate": { "type": "number" }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "run_macd_backtest".to_string(),
            description: "Run a MACD crossover backtest for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "fast": { "type": "integer", "minimum": 1 },
                    "slow": { "type": "integer", "minimum": 1 },
                    "signal": { "type": "integer", "minimum": 1 },
                    "initial_capital": { "type": "number", "minimum": 0.0 },
                    "commission_rate": { "type": "number", "minimum": 0.0 },
                    "periods_per_year": { "type": "integer", "minimum": 1 },
                    "risk_free_rate": { "type": "number" }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "run_bollinger_backtest".to_string(),
            description: "Run a Bollinger Bands mean-reversion backtest for a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "period": { "type": "integer", "minimum": 1 },
                    "std_dev": { "type": "integer", "minimum": 1 },
                    "initial_capital": { "type": "number", "minimum": 0.0 },
                    "commission_rate": { "type": "number", "minimum": 0.0 },
                    "periods_per_year": { "type": "integer", "minimum": 1 },
                    "risk_free_rate": { "type": "number" }
                },
                "required": ["ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "run_portfolio_backtest".to_string(),
            description: "Run a portfolio backtest across multiple strategy allocations."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "allocations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "ticker": { "type": "string" },
                                "strategy": { "type": "string" },
                                "params": { "type": "object" },
                                "weight": { "type": "number" }
                            },
                            "required": ["ticker", "strategy", "weight"]
                        }
                    },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "initial_capital": { "type": "number", "minimum": 0.0 },
                    "commission_rate": { "type": "number", "minimum": 0.0 },
                    "periods_per_year": { "type": "integer", "minimum": 1 },
                    "risk_free_rate": { "type": "number" }
                },
                "required": ["allocations", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "run_pair_backtest".to_string(),
            description: "Run a pairs-trading backtest between two tickers.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "leg1_ticker": { "type": "string" },
                    "leg2_ticker": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "lookback": { "type": "integer", "minimum": 1 },
                    "entry_threshold": { "type": "number" },
                    "exit_threshold": { "type": "number" },
                    "initial_capital": { "type": "number", "minimum": 0.0 },
                    "commission_rate": { "type": "number", "minimum": 0.0 },
                    "periods_per_year": { "type": "integer", "minimum": 1 },
                    "risk_free_rate": { "type": "number" }
                },
                "required": ["leg1_ticker", "leg2_ticker", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "run_walk_forward".to_string(),
            description: "Run walk-forward optimization for a strategy on a ticker.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "strategy": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "train_size": { "type": "integer", "minimum": 1 },
                    "test_size": { "type": "integer", "minimum": 1 },
                    "param_grid": { "type": "object" },
                    "metric": { "type": "string", "enum": ["total_return", "sharpe", "win_rate"] },
                    "initial_capital": { "type": "number", "minimum": 0.0 },
                    "commission_rate": { "type": "number", "minimum": 0.0 },
                    "periods_per_year": { "type": "integer", "minimum": 1 },
                    "risk_free_rate": { "type": "number" }
                },
                "required": ["ticker", "strategy", "start", "end", "train_size", "test_size", "param_grid"]
            }),
        },
        ToolDefinition {
            name: "run_monte_carlo".to_string(),
            description: "Run a Monte Carlo simulation over a strategy's trades.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "strategy": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "params": { "type": "object" },
                    "simulations": { "type": "integer", "minimum": 1 },
                    "seed": { "type": "integer" },
                    "initial_capital": { "type": "number", "minimum": 0.0 },
                    "commission_rate": { "type": "number", "minimum": 0.0 },
                    "periods_per_year": { "type": "integer", "minimum": 1 },
                    "risk_free_rate": { "type": "number" }
                },
                "required": ["ticker", "strategy", "start", "end"]
            }),
        },
        ToolDefinition {
            name: "run_robustness".to_string(),
            description: "Run robustness analysis by perturbing strategy parameters.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "strategy": { "type": "string" },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" },
                    "base_params": { "type": "object" },
                    "deltas": { "type": "object" },
                    "initial_capital": { "type": "number", "minimum": 0.0 },
                    "commission_rate": { "type": "number", "minimum": 0.0 },
                    "periods_per_year": { "type": "integer", "minimum": 1 },
                    "risk_free_rate": { "type": "number" }
                },
                "required": ["ticker", "strategy", "start", "end", "base_params", "deltas"]
            }),
        },
        // portfolio
        ToolDefinition {
            name: "optimize_mean_variance".to_string(),
            description: "Run mean-variance portfolio optimization on labelled return series."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "returns": { "type": "object", "additionalProperties": { "type": "array", "items": { "type": "number" } } },
                    "risk_free_rate": { "type": "number" },
                    "target_return": { "type": "number" }
                },
                "required": ["returns"]
            }),
        },
        ToolDefinition {
            name: "optimize_risk_parity".to_string(),
            description: "Compute inverse-volatility risk-parity weights.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "returns": { "type": "object", "additionalProperties": { "type": "array", "items": { "type": "number" } } }
                },
                "required": ["returns"]
            }),
        },
        ToolDefinition {
            name: "optimize_black_litterman".to_string(),
            description: "Run Black-Litterman portfolio optimization with expert views."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "returns": { "type": "object", "additionalProperties": { "type": "array", "items": { "type": "number" } } },
                    "market_caps": { "type": "object", "additionalProperties": { "type": "number" } },
                    "views": { "type": "object", "additionalProperties": { "type": "number" } },
                    "tau": { "type": "number", "minimum": 0.0 },
                    "risk_aversion": { "type": "number", "minimum": 0.0 }
                },
                "required": ["returns", "market_caps", "views", "tau", "risk_aversion"]
            }),
        },
        // screener
        ToolDefinition {
            name: "screen_fundamentals".to_string(),
            description: "Screen a fundamental universe with filters.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "filters": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "field": { "type": "string", "enum": ["pe", "market_cap", "pb", "dividend_yield", "eps_growth", "debt_to_equity", "roe"] },
                                "comparator": { "type": "string", "enum": ["lt", "gt"] },
                                "value": { "type": "number" }
                            },
                            "required": ["field", "comparator", "value"]
                        }
                    }
                },
                "required": ["filters"]
            }),
        },
        ToolDefinition {
            name: "screen_with_indicators".to_string(),
            description: "Screen a fundamental universe and apply technical indicator filters."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "filters": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "field": { "type": "string", "enum": ["pe", "market_cap", "pb", "dividend_yield", "eps_growth", "debt_to_equity", "roe"] },
                                "comparator": { "type": "string", "enum": ["lt", "gt"] },
                                "value": { "type": "number" }
                            },
                            "required": ["field", "comparator", "value"]
                        }
                    },
                    "indicator_filters": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "indicator": { "type": "string" },
                                "params": { "type": "object" },
                                "comparator": { "type": "string", "enum": ["lt", "gt"] },
                                "threshold": { "type": "number" }
                            },
                            "required": ["indicator", "comparator", "threshold"]
                        }
                    },
                    "start": { "type": "string", "format": "date" },
                    "end": { "type": "string", "format": "date" },
                    "interval": { "type": "string", "enum": ["daily", "weekly", "monthly"] },
                    "provider": { "type": "string" }
                },
                "required": ["filters", "indicator_filters", "start", "end"]
            }),
        },
        // audit
        ToolDefinition {
            name: "audit_verify".to_string(),
            description: "Placeholder: verify an audit trail.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "trail_id": { "type": "string" }
                },
                "required": ["trail_id"]
            }),
        },
        ToolDefinition {
            name: "audit_replay".to_string(),
            description: "Placeholder: replay an audit trail.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "trail_id": { "type": "string" }
                },
                "required": ["trail_id"]
            }),
        },
        // utility
        ToolDefinition {
            name: "health".to_string(),
            description: "Return agent health status.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDefinition {
            name: "list_tools".to_string(),
            description: "List all registered tool names.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
});

/// Returns all registered tool definitions.
pub fn list() -> &'static [ToolDefinition] {
    &TOOLS
}

/// Finds a tool definition by name.
pub fn find(name: &str) -> Option<&'static ToolDefinition> {
    TOOLS.iter().find(|t| t.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_expected_tools() {
        let tools = list();
        assert!(
            tools.len() >= 42,
            "expected at least 42 tools, got {}",
            tools.len()
        );
        assert!(find("compute_sma").is_some());
        assert!(find("black_scholes").is_some());
        assert!(find("list_tools").is_some());
    }
}
