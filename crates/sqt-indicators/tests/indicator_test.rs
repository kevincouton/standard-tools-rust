//! Unit tests for the `sqt-indicators` crate.

use chrono::NaiveDate;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use sqt_core::{Ohlcv, Ticker};
use sqt_indicators::IndicatorCalculator;
use std::collections::HashMap;

fn bar(date: &str, open: f64, high: f64, low: f64, close: f64, volume: i64) -> Ohlcv {
    Ohlcv::try_new(
        Ticker::new("TEST"),
        NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
        Decimal::from_f64(open).unwrap(),
        Decimal::from_f64(high).unwrap(),
        Decimal::from_f64(low).unwrap(),
        Decimal::from_f64(close).unwrap(),
        volume,
    )
    .unwrap()
}

fn assert_decimal_eq(actual: Decimal, expected: f64, epsilon: f64) {
    let expected_dec = Decimal::from_f64(expected).unwrap();
    let diff = (actual - expected_dec).abs();
    let eps = Decimal::from_f64(epsilon).unwrap();
    assert!(diff < eps, "expected {expected}, got {actual}, diff {diff}");
}

#[test]
fn sma_calculates_simple_moving_average() {
    let series = vec![
        bar("2024-01-01", 10.0, 12.0, 9.0, 10.0, 100),
        bar("2024-01-02", 11.0, 13.0, 10.0, 11.0, 100),
        bar("2024-01-03", 12.0, 14.0, 11.0, 12.0, 100),
        bar("2024-01-04", 13.0, 15.0, 12.0, 13.0, 100),
        bar("2024-01-05", 14.0, 16.0, 13.0, 14.0, 100),
    ];
    let mut params = HashMap::new();
    params.insert("period".to_string(), "3".to_string());

    let result = IndicatorCalculator::calculate("sma", &series, &params).unwrap();

    assert_eq!(result.name, "sma");
    assert!(result.extra_series.is_empty());
    assert_eq!(result.values.len(), 5);
    assert!(result.values[0].1.is_none());
    assert!(result.values[1].1.is_none());
    assert_decimal_eq(result.values[2].1.unwrap(), 11.0, 1e-10);
    assert_decimal_eq(result.values[3].1.unwrap(), 12.0, 1e-10);
    assert_decimal_eq(result.values[4].1.unwrap(), 13.0, 1e-10);
}

#[test]
fn ema_calculates_exponential_moving_average() {
    let series = vec![
        bar("2024-01-01", 10.0, 12.0, 9.0, 10.0, 100),
        bar("2024-01-02", 11.0, 13.0, 10.0, 11.0, 100),
        bar("2024-01-03", 12.0, 14.0, 11.0, 12.0, 100),
        bar("2024-01-04", 13.0, 15.0, 12.0, 13.0, 100),
        bar("2024-01-05", 14.0, 16.0, 13.0, 14.0, 100),
    ];
    let mut params = HashMap::new();
    params.insert("period".to_string(), "3".to_string());

    let result = IndicatorCalculator::calculate("ema", &series, &params).unwrap();

    assert_eq!(result.name, "ema");
    assert!(result.extra_series.is_empty());
    assert!(result.values[0].1.is_none());
    assert!(result.values[1].1.is_none());
    // Seed SMA = 11; multiplier = 2/(3+1) = 0.5
    assert_decimal_eq(result.values[2].1.unwrap(), 11.0, 1e-10);
    assert_decimal_eq(result.values[3].1.unwrap(), 12.0, 1e-10);
    assert_decimal_eq(result.values[4].1.unwrap(), 13.0, 1e-10);
}

#[test]
fn rsi_calculates_relative_strength_index() {
    let series = vec![
        bar("2024-01-01", 10.0, 11.0, 9.0, 10.0, 100),
        bar("2024-01-02", 11.0, 12.0, 10.0, 11.0, 100),
        bar("2024-01-03", 12.0, 13.0, 11.0, 12.0, 100),
        bar("2024-01-04", 11.0, 12.0, 10.0, 11.0, 100),
        bar("2024-01-05", 12.0, 13.0, 11.0, 12.0, 100),
        bar("2024-01-06", 13.0, 14.0, 12.0, 13.0, 100),
        bar("2024-01-07", 12.0, 13.0, 11.0, 12.0, 100),
    ];
    let mut params = HashMap::new();
    params.insert("period".to_string(), "2".to_string());

    let result = IndicatorCalculator::calculate("rsi", &series, &params).unwrap();

    assert_eq!(result.name, "rsi");
    assert!(result.extra_series.is_empty());
    assert!(result.values[0].1.is_none());
    assert!(result.values[1].1.is_none());
    assert_decimal_eq(result.values[2].1.unwrap(), 100.0, 1e-10);
    assert_decimal_eq(result.values[3].1.unwrap(), 50.0, 1e-10);
    assert_decimal_eq(result.values[4].1.unwrap(), 75.0, 1e-10);
}

#[test]
fn macd_calculates_macd_line() {
    let series = vec![
        bar("2024-01-01", 10.0, 11.0, 9.0, 10.0, 100),
        bar("2024-01-02", 11.0, 12.0, 10.0, 11.0, 100),
        bar("2024-01-03", 12.0, 13.0, 11.0, 12.0, 100),
        bar("2024-01-04", 13.0, 14.0, 12.0, 13.0, 100),
        bar("2024-01-05", 14.0, 15.0, 13.0, 14.0, 100),
        bar("2024-01-06", 15.0, 16.0, 14.0, 15.0, 100),
        bar("2024-01-07", 16.0, 17.0, 15.0, 16.0, 100),
        bar("2024-01-08", 17.0, 18.0, 16.0, 17.0, 100),
    ];
    let mut params = HashMap::new();
    params.insert("fast".to_string(), "2".to_string());
    params.insert("slow".to_string(), "4".to_string());
    params.insert("signal".to_string(), "9".to_string());

    let result = IndicatorCalculator::calculate("macd", &series, &params).unwrap();

    assert_eq!(result.name, "macd");
    assert!(result.extra_series.contains_key("signal"));
    assert!(result.extra_series.contains_key("histogram"));
    // Slow EMA warms up at index 3.
    assert!(result.values[2].1.is_none());
    assert_decimal_eq(result.values[3].1.unwrap(), 1.0, 1e-10);
    assert_decimal_eq(result.values[4].1.unwrap(), 1.0, 1e-10);
}

#[test]
fn bollinger_bands_returns_middle_band() {
    let series = vec![
        bar("2024-01-01", 10.0, 12.0, 9.0, 10.0, 100),
        bar("2024-01-02", 11.0, 13.0, 10.0, 11.0, 100),
        bar("2024-01-03", 12.0, 14.0, 11.0, 12.0, 100),
        bar("2024-01-04", 13.0, 15.0, 12.0, 13.0, 100),
        bar("2024-01-05", 14.0, 16.0, 13.0, 14.0, 100),
    ];
    let mut params = HashMap::new();
    params.insert("period".to_string(), "2".to_string());
    params.insert("std_dev".to_string(), "2".to_string());

    let result = IndicatorCalculator::calculate("bollinger_bands", &series, &params).unwrap();

    assert_eq!(result.name, "bollinger_bands");
    assert!(result.extra_series.contains_key("upper"));
    assert!(result.extra_series.contains_key("lower"));
    assert!(result.values[0].1.is_none());
    assert_decimal_eq(result.values[1].1.unwrap(), 10.5, 1e-10);
    assert_decimal_eq(result.values[2].1.unwrap(), 11.5, 1e-10);
    assert_decimal_eq(result.values[3].1.unwrap(), 12.5, 1e-10);
    assert_decimal_eq(result.values[4].1.unwrap(), 13.5, 1e-10);

    // Upper and lower bands should bracket the middle band.
    assert!(result.extra_series["upper"][1].1.unwrap() >= result.values[1].1.unwrap());
    assert!(result.extra_series["lower"][1].1.unwrap() <= result.values[1].1.unwrap());
}

#[test]
fn atr_calculates_average_true_range() {
    let series = vec![
        bar("2024-01-01", 11.0, 12.0, 10.0, 11.0, 100),
        bar("2024-01-02", 12.0, 13.0, 11.0, 12.0, 200),
        bar("2024-01-03", 13.0, 14.0, 12.0, 13.0, 200),
        bar("2024-01-04", 14.0, 15.0, 13.0, 14.0, 200),
        bar("2024-01-05", 15.0, 16.0, 14.0, 15.0, 200),
    ];
    let mut params = HashMap::new();
    params.insert("period".to_string(), "2".to_string());

    let result = IndicatorCalculator::calculate("atr", &series, &params).unwrap();

    assert_eq!(result.name, "atr");
    assert!(result.extra_series.is_empty());
    assert!(result.values[0].1.is_none());
    assert!(result.values[1].1.is_none());
    assert_decimal_eq(result.values[2].1.unwrap(), 2.0, 1e-10);
    assert_decimal_eq(result.values[3].1.unwrap(), 2.0, 1e-10);
    assert_decimal_eq(result.values[4].1.unwrap(), 2.0, 1e-10);
}

#[test]
fn obv_calculates_on_balance_volume() {
    let series = vec![
        bar("2024-01-01", 10.0, 11.0, 9.0, 10.0, 100),
        bar("2024-01-02", 11.0, 12.0, 10.0, 11.0, 200),
        bar("2024-01-03", 10.0, 11.0, 9.0, 10.0, 150),
        bar("2024-01-04", 12.0, 13.0, 11.0, 12.0, 300),
    ];

    let result = IndicatorCalculator::calculate("obv", &series, &HashMap::new()).unwrap();

    assert_eq!(result.name, "obv");
    assert!(result.extra_series.is_empty());
    assert_decimal_eq(result.values[0].1.unwrap(), 100.0, 1e-10);
    assert_decimal_eq(result.values[1].1.unwrap(), 300.0, 1e-10);
    assert_decimal_eq(result.values[2].1.unwrap(), 150.0, 1e-10);
    assert_decimal_eq(result.values[3].1.unwrap(), 450.0, 1e-10);
}

#[test]
fn vwap_calculates_volume_weighted_average_price() {
    let series = vec![
        bar("2024-01-01", 11.0, 12.0, 10.0, 11.0, 100),
        bar("2024-01-02", 12.0, 13.0, 11.0, 12.0, 200),
        bar("2024-01-03", 13.0, 14.0, 12.0, 13.0, 100),
    ];

    let result = IndicatorCalculator::calculate("vwap", &series, &HashMap::new()).unwrap();

    assert_eq!(result.name, "vwap");
    assert!(result.extra_series.is_empty());
    assert_decimal_eq(result.values[0].1.unwrap(), 11.0, 1e-10);
    assert_decimal_eq(result.values[1].1.unwrap(), 3500.0 / 300.0, 1e-10);
    assert_decimal_eq(result.values[2].1.unwrap(), 4800.0 / 400.0, 1e-10);
}

#[test]
fn unknown_indicator_returns_invalid_command() {
    let series = vec![bar("2024-01-01", 10.0, 11.0, 9.0, 10.0, 100)];
    let err = IndicatorCalculator::calculate("unknown", &series, &HashMap::new()).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::InvalidCommand(_)));
}

#[test]
fn realistic_bollinger_bands_have_expected_bands() {
    // 20 days of close prices that trend slightly upward. Period 20 gives one
    // warmed value at the end.
    let mut series = Vec::new();
    for i in 0..20 {
        let close = 100.0 + i as f64 * 0.5 + (if i % 2 == 0 { 1.0 } else { -1.0 });
        let date = NaiveDate::from_ymd_opt(2024, 1, 1 + i as u32).unwrap();
        series.push(
            Ohlcv::try_new(
                Ticker::new("TEST"),
                date,
                Decimal::from_f64(close - 1.0).unwrap(),
                Decimal::from_f64(close + 1.0).unwrap(),
                Decimal::from_f64(close - 1.0).unwrap(),
                Decimal::from_f64(close).unwrap(),
                1000,
            )
            .unwrap(),
        );
    }
    let mut params = HashMap::new();
    params.insert("period".to_string(), "20".to_string());
    params.insert("std_dev".to_string(), "2".to_string());

    let result = IndicatorCalculator::calculate("bollinger_bands", &series, &params).unwrap();

    let last = result.values.len() - 1;
    assert!(result.values[last].1.is_some(), "middle band ready");
    let middle = result.values[last].1.unwrap();
    let upper = result.extra_series["upper"][last].1.unwrap();
    let lower = result.extra_series["lower"][last].1.unwrap();
    assert!(upper > middle, "upper above middle");
    assert!(lower < middle, "lower below middle");
    assert!((upper - middle) > Decimal::ZERO);
    assert!((middle - lower) > Decimal::ZERO);
}

#[test]
fn realistic_macd_signal_and_histogram_follow_macd() {
    // Build 40 days of trending prices so fast/slow EMAs separate.
    let mut series = Vec::new();
    for i in 0..40 {
        let close = 100.0 + i as f64 * 0.2;
        let date = NaiveDate::from_ymd_opt(2024, 1, 1 + (i % 28) as u32).unwrap();
        series.push(
            Ohlcv::try_new(
                Ticker::new("TEST"),
                date,
                Decimal::from_f64(close - 0.1).unwrap(),
                Decimal::from_f64(close + 0.1).unwrap(),
                Decimal::from_f64(close - 0.1).unwrap(),
                Decimal::from_f64(close).unwrap(),
                1000,
            )
            .unwrap(),
        );
    }
    let mut params = HashMap::new();
    params.insert("fast".to_string(), "12".to_string());
    params.insert("slow".to_string(), "26".to_string());
    params.insert("signal".to_string(), "9".to_string());

    let result = IndicatorCalculator::calculate("macd", &series, &params).unwrap();

    // Once the signal line is warmed, histogram = macd - signal.
    for i in 0..result.values.len() {
        let macd = result.values[i].1;
        let signal = result.extra_series["signal"][i].1;
        let histogram = result.extra_series["histogram"][i].1;
        match (macd, signal, histogram) {
            (Some(m), Some(s), Some(h)) => {
                let expected: f64 = (m - s).try_into().unwrap();
                assert_decimal_eq(h, expected, 1e-10);
            }
            _ => {
                assert!(
                    histogram.is_none(),
                    "histogram should be None until both macd and signal are ready"
                );
            }
        }
    }
}
