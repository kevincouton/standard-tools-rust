use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqt_core::{BarInterval, DateRange, Ohlcv, QuantError, Ticker};

#[test]
fn ticker_construction() {
    let t = Ticker::new("AAPL");
    assert_eq!(t.symbol, "AAPL");
    assert_eq!(t.exchange, None);

    let t = Ticker::new("AAPL").with_exchange("NASDAQ");
    assert_eq!(t.symbol, "AAPL");
    assert_eq!(t.exchange, Some("NASDAQ".to_string()));
}

#[test]
fn ticker_try_new_accepts_valid_symbol() {
    let t = Ticker::try_new("AAPL").unwrap();
    assert_eq!(t.symbol, "AAPL");
    assert_eq!(t.exchange, None);
}

#[test]
fn ticker_try_new_rejects_empty_symbol() {
    let err = Ticker::try_new("").unwrap_err();
    assert!(matches!(err, QuantError::InvalidCommand(_)));
    assert!(err.to_string().contains("ticker symbol must be non-empty"));
}

#[test]
fn ticker_try_new_rejects_whitespace_symbol() {
    let err = Ticker::try_new("   ").unwrap_err();
    assert!(matches!(err, QuantError::InvalidCommand(_)));
    assert!(err.to_string().contains("ticker symbol must be non-empty"));
}

#[test]
fn ticker_equality_and_hash() {
    let a = Ticker::new("AAPL").with_exchange("NASDAQ");
    let b = Ticker::new("AAPL").with_exchange("NASDAQ");
    let c = Ticker::new("AAPL");
    assert_eq!(a, b);
    assert_ne!(a, c);

    // Same symbol/exchange pair should hash identically.
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher_a = DefaultHasher::new();
    let mut hasher_b = DefaultHasher::new();
    a.hash(&mut hasher_a);
    b.hash(&mut hasher_b);
    assert_eq!(hasher_a.finish(), hasher_b.finish());
}

#[test]
fn date_range_valid() {
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    let range = DateRange::try_new(start, end).unwrap();
    assert_eq!(range.start, start);
    assert_eq!(range.end, end);
}

#[test]
fn date_range_valid_when_start_equals_end() {
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let range = DateRange::try_new(date, date).unwrap();
    assert_eq!(range.start, date);
    assert_eq!(range.end, date);
}

#[test]
fn date_range_invalid_when_start_after_end() {
    let start = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let err = DateRange::try_new(start, end).unwrap_err();
    assert!(matches!(err, QuantError::InvalidCommand(_)));
    assert!(err.to_string().contains("start date"));
}

#[test]
fn ohlcv_construction_and_serialization() {
    let ohlcv = Ohlcv {
        ticker: Ticker::new("AAPL"),
        date: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        open: Decimal::try_new(15000, 2).unwrap(),
        high: Decimal::try_new(15500, 2).unwrap(),
        low: Decimal::try_new(14900, 2).unwrap(),
        close: Decimal::try_new(15350, 2).unwrap(),
        volume: 1_000_000,
    };

    assert_eq!(ohlcv.ticker.symbol, "AAPL");
    assert_eq!(ohlcv.close, Decimal::try_new(15350, 2).unwrap());

    let json = serde_json::to_string(&ohlcv).unwrap();
    assert!(json.contains("AAPL"));
    assert!(json.contains("153.50"));

    let deserialized: Ohlcv = serde_json::from_str(&json).unwrap();
    assert_eq!(ohlcv, deserialized);
}

#[test]
fn ohlcv_try_new_accepts_valid_bar() {
    let ohlcv = Ohlcv::try_new(
        Ticker::new("AAPL"),
        NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        Decimal::try_new(15000, 2).unwrap(),
        Decimal::try_new(15500, 2).unwrap(),
        Decimal::try_new(14900, 2).unwrap(),
        Decimal::try_new(15350, 2).unwrap(),
        1_000_000,
    )
    .unwrap();
    assert_eq!(ohlcv.ticker.symbol, "AAPL");
}

#[test]
fn ohlcv_try_new_rejects_high_below_low() {
    let err = Ohlcv::try_new(
        Ticker::new("AAPL"),
        NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        Decimal::try_new(15000, 2).unwrap(),
        Decimal::try_new(14000, 2).unwrap(),
        Decimal::try_new(14900, 2).unwrap(),
        Decimal::try_new(15350, 2).unwrap(),
        1_000_000,
    )
    .unwrap_err();
    assert!(matches!(err, QuantError::InvalidCommand(_)));
    assert!(err.to_string().contains("high price"));
}

#[test]
fn ohlcv_try_new_rejects_negative_price() {
    let err = Ohlcv::try_new(
        Ticker::new("AAPL"),
        NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        Decimal::try_new(-100, 2).unwrap(),
        Decimal::try_new(15500, 2).unwrap(),
        Decimal::try_new(14900, 2).unwrap(),
        Decimal::try_new(15350, 2).unwrap(),
        1_000_000,
    )
    .unwrap_err();
    assert!(matches!(err, QuantError::InvalidCommand(_)));
    assert!(err.to_string().contains("open price"));
}

#[test]
fn ohlcv_try_new_rejects_negative_volume() {
    let err = Ohlcv::try_new(
        Ticker::new("AAPL"),
        NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        Decimal::try_new(15000, 2).unwrap(),
        Decimal::try_new(15500, 2).unwrap(),
        Decimal::try_new(14900, 2).unwrap(),
        Decimal::try_new(15350, 2).unwrap(),
        -1,
    )
    .unwrap_err();
    assert!(matches!(err, QuantError::InvalidCommand(_)));
    assert!(err.to_string().contains("volume"));
}

#[test]
fn bar_interval_default_and_serialization() {
    let default: BarInterval = Default::default();
    assert_eq!(default, BarInterval::Daily);

    let json = serde_json::to_string(&BarInterval::Weekly).unwrap();
    assert_eq!(json, "\"weekly\"");

    let deserialized: BarInterval = serde_json::from_str("\"monthly\"").unwrap();
    assert_eq!(deserialized, BarInterval::Monthly);
}

#[test]
fn bar_interval_deserializes_snake_case_variants() {
    assert_eq!(
        serde_json::from_str::<BarInterval>("\"daily\"").unwrap(),
        BarInterval::Daily
    );
    assert_eq!(
        serde_json::from_str::<BarInterval>("\"weekly\"").unwrap(),
        BarInterval::Weekly
    );
    assert_eq!(
        serde_json::from_str::<BarInterval>("\"monthly\"").unwrap(),
        BarInterval::Monthly
    );
}
