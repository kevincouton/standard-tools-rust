//! Integration tests for the `sqt-agent` crate.
//!
//! These tests exercise the tool registry and dispatcher against an in-memory
//! market-data provider so that no external network calls are required.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, NaiveDate};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde_json::json;

use sqt_agent::{registry, ToolCall, ToolDispatcher};
use sqt_analysis::AnalysisService;
use sqt_backtest::BacktestService;
use sqt_core::{BarInterval, DateRange, Ohlcv, QuantError, Result, Ticker};
use sqt_marketdata::{MarketDataCache, MarketDataProvider, MarketDataService, MokaMarketDataCache};
use sqt_portfolio::PortfolioService;
use sqt_screener::{HardcodedFundamentalProvider, ScreenerService};

/// In-memory market-data provider used only in tests.
struct InlineProvider {
    data: HashMap<String, Vec<Ohlcv>>,
}

#[async_trait]
impl MarketDataProvider for InlineProvider {
    fn name(&self) -> &'static str {
        "inline"
    }

    async fn fetch(
        &self,
        ticker: &Ticker,
        _range: DateRange,
        _interval: BarInterval,
    ) -> Result<Vec<Ohlcv>> {
        self.data
            .get(&ticker.symbol)
            .cloned()
            .ok_or_else(|| QuantError::NotFound(format!("no data for {}", ticker.symbol)))
    }
}

fn make_test_data() -> HashMap<String, Vec<Ohlcv>> {
    let mut data = HashMap::new();
    data.insert("AAPL".to_string(), make_ohlcv_series("AAPL", 100));
    data.insert("MSFT".to_string(), make_ohlcv_series("MSFT", 100));
    data
}

fn make_ohlcv_series(symbol: &str, n: usize) -> Vec<Ohlcv> {
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid date");
    (0..n)
        .map(|i| {
            let date = start + Duration::days(i as i64);
            let close = Decimal::from_f64(100.0 + i as f64 * 0.5).expect("valid decimal");
            let open = close - Decimal::from_f64(0.2).expect("valid decimal");
            let high = close + Decimal::from_f64(0.3).expect("valid decimal");
            let low = close - Decimal::from_f64(0.4).expect("valid decimal");
            Ohlcv::try_new(Ticker::new(symbol), date, open, high, low, close, 1_000_000)
                .expect("valid ohlcv")
        })
        .collect()
}

fn build_dispatcher() -> ToolDispatcher<HardcodedFundamentalProvider> {
    let cache: Arc<dyn MarketDataCache> = MokaMarketDataCache::new().into();
    let service = MarketDataService::new("inline", cache);
    service.register_provider(Arc::new(InlineProvider {
        data: make_test_data(),
    }));

    ToolDispatcher::new(
        Arc::new(service),
        BacktestService::new(),
        AnalysisService::new(),
        PortfolioService::new(),
        ScreenerService::new(HardcodedFundamentalProvider::new()),
    )
}

#[tokio::test]
async fn registry_has_at_least_42_tools() {
    assert!(
        registry::list().len() >= 42,
        "expected at least 42 tools, got {}",
        registry::list().len()
    );
}

#[tokio::test]
async fn dispatch_list_tools() {
    let dispatcher = build_dispatcher();
    let result = dispatcher
        .dispatch(ToolCall {
            name: "list_tools".to_string(),
            arguments: json!({}),
        })
        .await
        .expect("dispatch succeeds");

    assert!(result.error.is_none());
    let tools = result.output.as_array().expect("output is an array");
    assert!(tools.iter().any(|t| t.as_str() == Some("list_tools")));
}

#[tokio::test]
async fn dispatch_black_scholes() {
    let dispatcher = build_dispatcher();
    let result = dispatcher
        .dispatch(ToolCall {
            name: "black_scholes".to_string(),
            arguments: json!({
                "spot": 100.0,
                "strike": 100.0,
                "risk_free_rate": 0.05,
                "volatility": 0.2,
                "time_to_maturity": 1.0,
                "option_type": "call"
            }),
        })
        .await
        .expect("dispatch succeeds");

    assert!(result.error.is_none());
    let price = result
        .output
        .get("price")
        .and_then(|v| v.as_f64())
        .expect("price present");
    assert!(price > 0.0);
}

#[tokio::test]
async fn dispatch_compute_sma() {
    let dispatcher = build_dispatcher();
    let result = dispatcher
        .dispatch(ToolCall {
            name: "compute_sma".to_string(),
            arguments: json!({
                "ticker": "AAPL",
                "start": "2024-01-01",
                "end": "2024-04-09",
                "period": 5
            }),
        })
        .await
        .expect("dispatch succeeds");

    assert!(result.error.is_none());
    let values = result
        .output
        .get("values")
        .and_then(|v| v.as_array())
        .expect("values present");
    assert!(!values.is_empty());
}

#[tokio::test]
async fn dispatch_run_sma_backtest() {
    let dispatcher = build_dispatcher();
    let result = dispatcher
        .dispatch(ToolCall {
            name: "run_sma_backtest".to_string(),
            arguments: json!({
                "ticker": "AAPL",
                "start": "2024-01-01",
                "end": "2024-04-09",
                "fast": 5,
                "slow": 10
            }),
        })
        .await
        .expect("dispatch succeeds");

    assert!(result.error.is_none());
    assert!(result.output.get("total_return").is_some());
}

#[tokio::test]
async fn dispatch_optimize_mean_variance() {
    let dispatcher = build_dispatcher();
    let result = dispatcher
        .dispatch(ToolCall {
            name: "optimize_mean_variance".to_string(),
            arguments: json!({
                "returns": {
                    "AAPL": [0.01, 0.02, -0.01, 0.015, 0.005, 0.01, -0.005, 0.02, 0.01, -0.01, 0.015, 0.005],
                    "MSFT": [0.005, 0.01, 0.0, 0.02, -0.005, 0.015, 0.0, 0.01, 0.005, 0.0, 0.02, -0.005]
                },
                "risk_free_rate": 0.0
            }),
        })
        .await
        .expect("dispatch succeeds");

    assert!(result.error.is_none());
    let weights = result
        .output
        .get("weights")
        .and_then(|v| v.as_object())
        .expect("weights present");
    let total: f64 = weights
        .values()
        .map(|v| v.as_f64().expect("weight is a number"))
        .sum();
    assert!((total - 1.0).abs() < 1e-6);
}

#[tokio::test]
async fn dispatch_screen_fundamentals() {
    let dispatcher = build_dispatcher();
    let result = dispatcher
        .dispatch(ToolCall {
            name: "screen_fundamentals".to_string(),
            arguments: json!({
                "filters": [{ "field": "pe", "comparator": "lt", "value": 30.0 }]
            }),
        })
        .await
        .expect("dispatch succeeds");

    assert!(result.error.is_none());
    let results = result.output.as_array().expect("results is an array");
    assert!(!results.is_empty());

    let tickers: Vec<String> = results
        .iter()
        .map(|r| {
            r.get("ticker")
                .and_then(|v| v.as_str())
                .expect("ticker present")
                .to_string()
        })
        .collect();
    assert!(tickers.contains(&"AAPL".to_string()));
}

#[tokio::test]
async fn dispatch_unknown_tool_returns_error() {
    let dispatcher = build_dispatcher();
    let result = dispatcher
        .dispatch(ToolCall {
            name: "not_a_tool".to_string(),
            arguments: json!({}),
        })
        .await;

    assert!(
        matches!(result, Err(QuantError::InvalidCommand(_))),
        "expected InvalidCommand for unknown tool"
    );
}

#[tokio::test]
async fn dispatch_compute_sharpe() {
    let dispatcher = build_dispatcher();
    let result = dispatcher
        .dispatch(ToolCall {
            name: "compute_sharpe".to_string(),
            arguments: json!({
                "ticker": "AAPL",
                "start": "2024-01-01",
                "end": "2024-04-09",
                "risk_free_rate": 0.0,
                "periods_per_year": 252
            }),
        })
        .await
        .expect("dispatch succeeds");

    assert!(
        result.error.is_none(),
        "expected no error, got: {:?}",
        result.error
    );
    assert!(result.output.get("sharpe").is_some());
}

#[tokio::test]
async fn dispatch_correlation_matrix() {
    let dispatcher = build_dispatcher();
    let result = dispatcher
        .dispatch(ToolCall {
            name: "correlation_matrix".to_string(),
            arguments: json!({
                "tickers": ["AAPL", "MSFT"],
                "start": "2024-01-01",
                "end": "2024-04-09"
            }),
        })
        .await
        .expect("dispatch succeeds");

    assert!(result.error.is_none());
    let matrix = result
        .output
        .get("matrix")
        .and_then(|v| v.as_array())
        .expect("matrix present");
    assert_eq!(matrix.len(), 2);
}

#[tokio::test]
async fn dispatch_optimize_risk_parity() {
    let dispatcher = build_dispatcher();
    let result = dispatcher
        .dispatch(ToolCall {
            name: "optimize_risk_parity".to_string(),
            arguments: json!({
                "returns": {
                    "AAPL": [0.01, 0.02, -0.01, 0.015, 0.005, 0.01, -0.005, 0.02, 0.01, -0.01, 0.015, 0.005],
                    "MSFT": [0.005, 0.01, 0.0, 0.02, -0.005, 0.015, 0.0, 0.01, 0.005, 0.0, 0.02, -0.005]
                }
            }),
        })
        .await
        .expect("dispatch succeeds");

    assert!(result.error.is_none());
    let weights = result
        .output
        .get("weights")
        .and_then(|v| v.as_object())
        .expect("weights present");
    let total: f64 = weights
        .values()
        .map(|v| v.as_f64().expect("weight is a number"))
        .sum();
    assert!((total - 1.0).abs() < 1e-6);
}

#[tokio::test]
async fn dispatch_run_macd_backtest() {
    let dispatcher = build_dispatcher();
    let result = dispatcher
        .dispatch(ToolCall {
            name: "run_macd_backtest".to_string(),
            arguments: json!({
                "ticker": "AAPL",
                "start": "2024-01-01",
                "end": "2024-04-09",
                "fast": 5,
                "slow": 10,
                "signal": 3
            }),
        })
        .await
        .expect("dispatch succeeds");

    assert!(result.error.is_none());
    assert!(result.output.get("total_return").is_some());
}

#[tokio::test]
async fn dispatch_missing_required_argument_returns_tool_error() {
    let dispatcher = build_dispatcher();
    let result = dispatcher
        .dispatch(ToolCall {
            name: "compute_sma".to_string(),
            arguments: json!({
                "ticker": "AAPL",
                "end": "2024-04-09"
            }),
        })
        .await
        .expect("dispatch succeeds for known tool");

    assert!(
        result.error.is_some(),
        "expected error field populated for missing start date"
    );
}

#[tokio::test]
async fn dispatch_invalid_interval_returns_tool_error() {
    let dispatcher = build_dispatcher();
    let result = dispatcher
        .dispatch(ToolCall {
            name: "compute_sma".to_string(),
            arguments: json!({
                "ticker": "AAPL",
                "start": "2024-01-01",
                "end": "2024-04-09",
                "interval": "hourly"
            }),
        })
        .await
        .expect("dispatch succeeds for known tool");

    assert!(
        result.error.is_some(),
        "expected error field populated for invalid interval"
    );
}

#[tokio::test]
async fn dispatch_invalid_black_scholes_returns_tool_error() {
    let dispatcher = build_dispatcher();
    let result = dispatcher
        .dispatch(ToolCall {
            name: "black_scholes".to_string(),
            arguments: json!({
                "spot": -100.0,
                "strike": 100.0,
                "risk_free_rate": 0.05,
                "volatility": 0.2,
                "time_to_maturity": 1.0,
                "option_type": "call"
            }),
        })
        .await
        .expect("dispatch succeeds for known tool");

    assert!(
        result.error.is_some(),
        "expected error field populated for invalid Black-Scholes input"
    );
}
