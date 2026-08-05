//! Integration tests for the `sqt-screener` crate.

use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, Ticker};
use sqt_screener::{
    Comparator, FundamentalData, FundamentalFilter, FundamentalProvider,
    HardcodedFundamentalProvider, IndicatorFilter, ScreenerService,
};

fn make_bar(ticker: &str, date: NaiveDate, close: f64) -> Ohlcv {
    let close_dec = Decimal::from_f64_retain(close).unwrap();
    Ohlcv::try_new(
        Ticker::new(ticker),
        date,
        close_dec,
        close_dec,
        close_dec,
        close_dec,
        1_000,
    )
    .unwrap()
}

fn rising_series(ticker: &str, days: usize, start_price: f64) -> Vec<Ohlcv> {
    let base = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    (0..days)
        .map(|i| {
            let price = start_price + i as f64 * 1.0;
            make_bar(ticker, base + chrono::Duration::days(i as i64), price)
        })
        .collect()
}

#[test]
fn hardcoded_provider_returns_ten_tickers() {
    let provider = HardcodedFundamentalProvider::new();
    let data = provider.get_all().unwrap();
    assert_eq!(data.len(), 10);
    assert!(data.iter().any(|d| d.ticker == "AAPL"));
    assert!(data.iter().any(|d| d.ticker == "TSLA"));
}

#[test]
fn fundamental_screen_finds_value_stocks() {
    let provider = HardcodedFundamentalProvider::new();
    let service = ScreenerService::new(provider);

    let filters = vec![FundamentalFilter::PeLt(15.0)];
    let results = service.screen(&filters).unwrap();

    let tickers: Vec<&str> = results.iter().map(|d| d.ticker.as_str()).collect();
    assert!(tickers.contains(&"JPM"));
    assert!(tickers.contains(&"XOM"));
    assert!(tickers.contains(&"PFE"));
    assert!(tickers.contains(&"BAC"));
    assert!(!tickers.contains(&"TSLA"));
    assert!(!tickers.contains(&"AAPL"));
}

#[test]
fn screen_with_indicators_filters_by_rsi() {
    let provider = HardcodedFundamentalProvider::new();
    let service = ScreenerService::new(provider);

    let mut ohlcv_data: HashMap<String, Vec<Ohlcv>> = HashMap::new();
    // AAPL has a strongly rising series -> RSI should be near 100.
    ohlcv_data.insert("AAPL".to_string(), rising_series("AAPL", 30, 100.0));

    let fundamental_filters = vec![];
    let indicator_filters = vec![IndicatorFilter {
        indicator: "rsi".to_string(),
        params: [("period".to_string(), "14".to_string())]
            .into_iter()
            .collect(),
        comparator: Comparator::Gt,
        threshold: 70.0,
    }];

    let results = service
        .screen_with_indicators(&fundamental_filters, &indicator_filters, &ohlcv_data)
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].ticker, "AAPL");
}

#[test]
fn combined_fundamental_and_indicator_screen() {
    let provider = HardcodedFundamentalProvider::new();
    let service = ScreenerService::new(provider);

    let mut ohlcv_data: HashMap<String, Vec<Ohlcv>> = HashMap::new();
    ohlcv_data.insert("AAPL".to_string(), rising_series("AAPL", 30, 100.0));

    // AAPL has P/E > 15, so it should be filtered out by the fundamental rule
    // even though its RSI is high.
    let fundamental_filters = vec![FundamentalFilter::PeLt(15.0)];
    let indicator_filters = vec![IndicatorFilter {
        indicator: "rsi".to_string(),
        params: [("period".to_string(), "14".to_string())]
            .into_iter()
            .collect(),
        comparator: Comparator::Gt,
        threshold: 70.0,
    }];

    let results = service
        .screen_with_indicators(&fundamental_filters, &indicator_filters, &ohlcv_data)
        .unwrap();

    assert!(results.is_empty());
}

#[derive(Debug, Clone, Default)]
struct EmptyProvider;

impl FundamentalProvider for EmptyProvider {
    fn get_all(&self) -> sqt_core::Result<Vec<FundamentalData>> {
        Ok(vec![])
    }
}

#[test]
fn screen_with_empty_provider_returns_empty() {
    let service = ScreenerService::new(EmptyProvider);
    let results = service.screen(&[FundamentalFilter::PeLt(15.0)]).unwrap();
    assert!(results.is_empty());
}

#[test]
fn screen_with_invalid_indicator_name_propagates_error() {
    let provider = HardcodedFundamentalProvider::new();
    let service = ScreenerService::new(provider);

    let mut ohlcv_data: HashMap<String, Vec<Ohlcv>> = HashMap::new();
    ohlcv_data.insert("AAPL".to_string(), rising_series("AAPL", 30, 100.0));

    let indicator_filters = vec![IndicatorFilter {
        indicator: "not-a-real-indicator".to_string(),
        params: HashMap::new(),
        comparator: Comparator::Gt,
        threshold: 0.0,
    }];

    let err = service
        .screen_with_indicators(&[], &indicator_filters, &ohlcv_data)
        .unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::InvalidCommand(_)));
}

#[test]
fn fundamental_filter_excludes_nan_metrics() {
    let data = FundamentalData {
        ticker: "NANCO".to_string(),
        market_cap: 1.0,
        pe_ratio: f64::NAN,
        pb_ratio: 1.0,
        dividend_yield: 0.0,
        eps_growth: 0.0,
        debt_to_equity: 0.0,
        roe: 0.0,
    };

    // Any filter that depends on the NaN metric should exclude the security.
    assert!(!FundamentalFilter::PeLt(100.0).apply(&data));
    assert!(!FundamentalFilter::PeGt(0.0).apply(&data));
}
