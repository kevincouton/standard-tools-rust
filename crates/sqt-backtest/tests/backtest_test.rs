//! Integration tests for the `sqt-backtest` crate.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::NaiveDate;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use sqt_backtest::{
    BacktestConfig, BacktestEngine, BacktestService, OptimizationMetric, PairBacktestConfig,
    PortfolioAllocation, Signal, SignalResult, Strategy, WalkForwardConfig,
};
use sqt_core::{Ohlcv, QuantError, Ticker};

fn ticker() -> Ticker {
    Ticker::new("TEST")
}

fn make_series(start_price: f64, increments: &[f64]) -> Vec<Ohlcv> {
    let base = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let mut price = start_price;
    increments
        .iter()
        .enumerate()
        .map(|(i, inc)| {
            let open = price;
            let close = price + inc;
            let high = open.max(close) + inc.abs() * 0.1;
            let low = open.min(close) - inc.abs() * 0.1;
            price = close;
            Ohlcv::try_new(
                ticker(),
                base + chrono::Duration::days(i as i64),
                Decimal::from_f64(open).unwrap(),
                Decimal::from_f64(high).unwrap(),
                Decimal::from_f64(low).unwrap(),
                Decimal::from_f64(close).unwrap(),
                1_000_000,
            )
            .unwrap()
        })
        .collect()
}

fn trending_series(n: usize) -> Vec<Ohlcv> {
    let mut incs = Vec::with_capacity(n);
    for i in 0..n {
        if i % 20 < 10 {
            incs.push(1.0);
        } else {
            incs.push(-1.0);
        }
    }
    make_series(100.0, &incs)
}

fn oscillating_series(n: usize) -> Vec<Ohlcv> {
    // Alternating strong up/down legs to push RSI into overbought/oversold
    // territory and to pierce Bollinger Bands.
    let mut incs = Vec::with_capacity(n);
    for i in 0..n {
        let leg = i / 15;
        if leg % 2 == 0 {
            incs.push(4.0);
        } else {
            incs.push(-4.0);
        }
    }
    make_series(100.0, &incs)
}

fn paired_series(n: usize) -> (Vec<Ohlcv>, Vec<Ohlcv>) {
    let base = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let mut leg1 = Vec::with_capacity(n);
    let mut leg2 = Vec::with_capacity(n);
    let mut price1 = 100.0;
    let mut price2 = 50.0;
    for i in 0..n {
        let noise1 = (i as f64 * 0.1).sin();
        let noise2 = (i as f64 * 0.1).cos();
        price1 += noise1 * 0.5;
        price2 += noise1 * 0.25 + noise2 * 0.1;
        let high1 = price1 + 0.1;
        let low1 = price1 - 0.1;
        let high2 = price2 + 0.1;
        let low2 = price2 - 0.1;
        leg1.push(
            Ohlcv::try_new(
                ticker(),
                base + chrono::Duration::days(i as i64),
                Decimal::from_f64(price1).unwrap(),
                Decimal::from_f64(high1).unwrap(),
                Decimal::from_f64(low1).unwrap(),
                Decimal::from_f64(price1).unwrap(),
                1_000_000,
            )
            .unwrap(),
        );
        leg2.push(
            Ohlcv::try_new(
                ticker(),
                base + chrono::Duration::days(i as i64),
                Decimal::from_f64(price2).unwrap(),
                Decimal::from_f64(high2).unwrap(),
                Decimal::from_f64(low2).unwrap(),
                Decimal::from_f64(price2).unwrap(),
                1_000_000,
            )
            .unwrap(),
        );
    }
    (leg1, leg2)
}

fn config() -> BacktestConfig {
    BacktestConfig {
        initial_capital: Decimal::from(100_000),
        commission_rate: Some(Decimal::from_f64(0.001).unwrap()),
        periods_per_year: 252,
        risk_free_rate: 0.0,
    }
}

#[test]
fn sma_crossover_produces_trades() {
    let service = BacktestService::new();
    let series = trending_series(80);
    let mut params = HashMap::new();
    params.insert("fast".to_string(), "5".to_string());
    params.insert("slow".to_string(), "20".to_string());

    let result = service
        .run_single_strategy("sma_crossover", &series, params, config())
        .unwrap();

    assert!(
        !result.trades.is_empty(),
        "SMA crossover should produce trades"
    );
    assert_eq!(result.equity_curve.len(), series.len());
    assert!(result.number_of_trades > 0);
}

#[test]
fn rsi_mean_reversion_produces_trades() {
    let service = BacktestService::new();
    let series = oscillating_series(80);
    let mut params = HashMap::new();
    params.insert("period".to_string(), "14".to_string());
    params.insert("oversold".to_string(), "30".to_string());
    params.insert("overbought".to_string(), "70".to_string());

    let result = service
        .run_single_strategy("rsi_mean_reversion", &series, params, config())
        .unwrap();

    assert!(
        !result.trades.is_empty(),
        "RSI strategy should produce trades"
    );
    assert_eq!(result.equity_curve.len(), series.len());
}

#[test]
fn macd_crossover_produces_trades() {
    let service = BacktestService::new();
    let series = trending_series(80);
    let params = HashMap::new();

    let result = service
        .run_single_strategy("macd_crossover", &series, params, config())
        .unwrap();

    assert!(
        !result.trades.is_empty(),
        "MACD strategy should produce trades"
    );
    assert_eq!(result.equity_curve.len(), series.len());
}

fn spiky_series(n: usize) -> Vec<Ohlcv> {
    // Flat for 20 bars, then large jumps to pierce Bollinger Bands.
    let mut incs = Vec::with_capacity(n);
    for i in 0..n {
        if i < 20 {
            incs.push(0.0);
        } else {
            let cycle = (i - 20) / 10;
            if cycle % 2 == 0 {
                incs.push(15.0);
            } else {
                incs.push(-15.0);
            }
        }
    }
    make_series(100.0, &incs)
}

#[test]
fn bollinger_reversion_produces_trades() {
    let service = BacktestService::new();
    let series = spiky_series(80);
    let params = HashMap::new();

    let result = service
        .run_single_strategy("bollinger_reversion", &series, params, config())
        .unwrap();

    assert!(
        !result.trades.is_empty(),
        "Bollinger strategy should produce trades"
    );
    assert_eq!(result.equity_curve.len(), series.len());
}

#[test]
fn portfolio_backtest_aggregates_equity() {
    let service = BacktestService::new();
    let series1 = trending_series(60);
    let series2 = oscillating_series(60);

    let allocations = vec![
        PortfolioAllocation {
            label: "sma".to_string(),
            series: series1,
            strategy: Arc::new(sqt_backtest::SmaCrossover),
            params: {
                let mut p = HashMap::new();
                p.insert("fast".to_string(), "5".to_string());
                p.insert("slow".to_string(), "20".to_string());
                p
            },
            weight: 0.5,
        },
        PortfolioAllocation {
            label: "rsi".to_string(),
            series: series2,
            strategy: Arc::new(sqt_backtest::RsiMeanReversion),
            params: {
                let mut p = HashMap::new();
                p.insert("period".to_string(), "14".to_string());
                p.insert("oversold".to_string(), "30".to_string());
                p.insert("overbought".to_string(), "70".to_string());
                p
            },
            weight: 0.5,
        },
    ];

    let result = service.run_portfolio(allocations, config()).unwrap();
    assert!(!result.equity_curve.is_empty());
    assert!(result.per_asset.contains_key("sma"));
    assert!(result.per_asset.contains_key("rsi"));
}

#[test]
fn pair_backtest_produces_result() {
    let service = BacktestService::new();
    let (leg1, leg2) = paired_series(120);

    let pair_config = PairBacktestConfig {
        backtest: config(),
        lookback: 60,
        entry_threshold: 1.5,
        exit_threshold: 0.25,
    };

    let result = service.run_pair(&leg1, &leg2, pair_config).unwrap();
    assert_eq!(result.equity_curve.len(), leg1.len());
}

#[test]
fn walk_forward_runs_out_of_sample_windows() {
    let service = BacktestService::new();
    let series = trending_series(120);
    let mut param_grid = HashMap::new();
    param_grid.insert("fast".to_string(), vec!["5".to_string(), "10".to_string()]);
    param_grid.insert("slow".to_string(), vec!["20".to_string(), "30".to_string()]);

    let wf_config = WalkForwardConfig {
        train_size: 50,
        test_size: 20,
        param_grid,
        metric: OptimizationMetric::TotalReturn,
        backtest: config(),
    };

    let result = service
        .run_walk_forward("sma_crossover", &series, wf_config)
        .unwrap();
    assert!(!result.equity_curve.is_empty());
    assert!(!result.selected_params.is_empty());
}

#[test]
fn monte_carlo_produces_confidence_intervals() {
    let service = BacktestService::new();
    let series = trending_series(80);
    let params = {
        let mut p = HashMap::new();
        p.insert("fast".to_string(), "5".to_string());
        p.insert("slow".to_string(), "20".to_string());
        p
    };

    let result = service
        .run_monte_carlo(
            "sma_crossover",
            &series,
            params,
            config(),
            Some(100),
            Some(42),
        )
        .unwrap();

    assert_eq!(result.simulations, 100);
    assert!(result.final_equity_ci.lower <= result.final_equity_ci.upper);
    assert!(result.max_drawdown_ci.lower <= result.max_drawdown_ci.upper);
}

#[test]
fn robustness_perturbs_parameters() {
    let service = BacktestService::new();
    let series = trending_series(80);
    let base_params = {
        let mut p = HashMap::new();
        p.insert("fast".to_string(), "10".to_string());
        p.insert("slow".to_string(), "30".to_string());
        p
    };
    let mut deltas = HashMap::new();
    deltas.insert("fast".to_string(), 0.2);
    deltas.insert("slow".to_string(), 0.2);

    let result = service
        .run_robustness("sma_crossover", &series, base_params, deltas, config())
        .unwrap();

    assert!(!result.perturbations.is_empty());
    assert!(result.perturbations.len() >= 2);
}

// Deterministic strategies for calculator-style tests.

#[derive(Debug, Clone, Copy, Default)]
struct AlwaysBuy;

impl Strategy for AlwaysBuy {
    fn name(&self) -> &'static str {
        "always_buy"
    }

    fn signals(
        &self,
        series: &[Ohlcv],
        _params: &HashMap<String, String>,
    ) -> std::result::Result<Vec<SignalResult>, QuantError> {
        Ok(series
            .iter()
            .map(|bar| SignalResult {
                date: bar.date,
                signal: Signal::Buy,
            })
            .collect())
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct AlwaysSell;

impl Strategy for AlwaysSell {
    fn name(&self) -> &'static str {
        "always_sell"
    }

    fn signals(
        &self,
        series: &[Ohlcv],
        _params: &HashMap<String, String>,
    ) -> std::result::Result<Vec<SignalResult>, QuantError> {
        Ok(series
            .iter()
            .map(|bar| SignalResult {
                date: bar.date,
                signal: Signal::Sell,
            })
            .collect())
    }
}

#[test]
fn long_trade_pnl_and_commission_are_correct() {
    // Two-bar series: buy at open of bar 2, mark at close of bar 2, then close
    // at the last close.
    let series = vec![
        make_bar(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(), 100.0, 101.0),
        make_bar(NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(), 101.0, 102.0),
    ];

    let config = BacktestConfig {
        initial_capital: Decimal::from(100_000),
        commission_rate: Some(Decimal::from_f64(0.001).unwrap()),
        periods_per_year: 252,
        risk_free_rate: 0.0,
    };

    let engine = BacktestEngine::new(Arc::new(AlwaysBuy), config);
    let result = engine.run(&series, &HashMap::new()).unwrap();

    // At bar 1 open (101) we buy quantity = 100000 / (101 * 1.001)
    // entry_notional = qty * 101, entry_commission = entry_notional * 0.001
    // cash after open = 100000 - entry_notional - entry_commission
    // At bar 1 close (102) equity = cash + qty * 102
    // At last close we close: cash = cash + qty * 102 - exit_commission
    let price = Decimal::from_f64(101.0).unwrap();
    let one_plus_commission = Decimal::from_f64(1.001).unwrap();
    let qty = Decimal::from(100_000) / (price * one_plus_commission);
    let entry_notional = qty * price;
    let commission = entry_notional * Decimal::from_f64(0.001).unwrap();
    let exit_price = Decimal::from_f64(102.0).unwrap();
    let exit_commission = qty * exit_price * Decimal::from_f64(0.001).unwrap();
    let expected_final =
        Decimal::from(100_000) - entry_notional - commission + qty * exit_price - exit_commission;

    let trade = result.trades.first().expect("one trade");
    assert_eq!(trade.side, sqt_backtest::TradeSide::Long);
    assert!(
        (trade.pnl - (qty * (exit_price - price) - commission - exit_commission)).abs()
            < Decimal::from_f64(1e-6).unwrap()
    );
    assert_eq!(result.equity_curve.len(), series.len());
    assert!(
        (result.final_equity().unwrap() - expected_final).abs() < Decimal::from_f64(1e-6).unwrap()
    );
}

#[test]
fn short_trade_pnl_is_negative_when_price_rises() {
    // Short at open of bar 2 (101), close at last close (102) -> loss.
    let series = vec![
        make_bar(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(), 100.0, 101.0),
        make_bar(NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(), 101.0, 102.0),
    ];

    let config = BacktestConfig {
        initial_capital: Decimal::from(100_000),
        commission_rate: Some(Decimal::from_f64(0.001).unwrap()),
        periods_per_year: 252,
        risk_free_rate: 0.0,
    };

    let engine = BacktestEngine::new(Arc::new(AlwaysSell), config);
    let result = engine.run(&series, &HashMap::new()).unwrap();

    let trade = result.trades.first().expect("one trade");
    assert_eq!(trade.side, sqt_backtest::TradeSide::Short);
    assert!(
        trade.pnl < Decimal::ZERO,
        "short should lose when price rises"
    );
}

#[test]
fn commission_reduces_final_equity() {
    let series = vec![
        make_bar(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(), 100.0, 100.0),
        make_bar(NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(), 100.0, 100.0),
    ];

    let config_no_commission = BacktestConfig {
        initial_capital: Decimal::from(100_000),
        commission_rate: None,
        periods_per_year: 252,
        risk_free_rate: 0.0,
    };
    let config_with_commission = BacktestConfig {
        initial_capital: Decimal::from(100_000),
        commission_rate: Some(Decimal::from_f64(0.01).unwrap()),
        periods_per_year: 252,
        risk_free_rate: 0.0,
    };

    let engine_no = BacktestEngine::new(Arc::new(AlwaysBuy), config_no_commission);
    let result_no = engine_no.run(&series, &HashMap::new()).unwrap();

    let engine_with = BacktestEngine::new(Arc::new(AlwaysBuy), config_with_commission);
    let result_with = engine_with.run(&series, &HashMap::new()).unwrap();

    assert!(
        result_with.final_equity().unwrap() < result_no.final_equity().unwrap(),
        "commission should reduce final equity"
    );
}

#[test]
fn empty_series_returns_error() {
    let config = BacktestConfig {
        initial_capital: Decimal::from(100_000),
        commission_rate: None,
        periods_per_year: 252,
        risk_free_rate: 0.0,
    };
    let engine = BacktestEngine::new(Arc::new(AlwaysBuy), config);
    let result = engine.run(&[], &HashMap::new());
    assert!(result.is_err());
}

fn make_bar(date: NaiveDate, open: f64, close: f64) -> Ohlcv {
    let high = open.max(close) + 0.01;
    let low = open.min(close) - 0.01;
    Ohlcv::try_new(
        ticker(),
        date,
        Decimal::from_f64(open).unwrap(),
        Decimal::from_f64(high).unwrap(),
        Decimal::from_f64(low).unwrap(),
        Decimal::from_f64(close).unwrap(),
        1_000_000,
    )
    .unwrap()
}
