//! Integration tests for the `sqt-marketdata` crate.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqt_core::{BarInterval, DateRange, Ohlcv, QuantError, Result, Ticker};
use sqt_marketdata::{
    MarketDataCache, MarketDataProvider, MarketDataService, MokaMarketDataCache,
    YahooFinanceProvider,
};
use tokio::sync::Mutex;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A simple in-memory cache for testing cache hit/miss behaviour.
#[derive(Clone)]
struct InMemoryCache {
    store: Arc<Mutex<HashMap<String, Vec<Ohlcv>>>>,
}

impl Default for InMemoryCache {
    fn default() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl MarketDataCache for InMemoryCache {
    async fn get(&self, key: &str) -> Option<Vec<Ohlcv>> {
        self.store.lock().await.get(key).cloned()
    }

    async fn put(&self, key: &str, series: Vec<Ohlcv>) {
        self.store.lock().await.insert(key.to_string(), series);
    }
}

/// A fake provider that returns a deterministic series and records call counts.
struct FakeProvider {
    name: &'static str,
    call_count: Arc<Mutex<usize>>,
}

impl FakeProvider {
    fn new(name: &'static str) -> (Self, Arc<Mutex<usize>>) {
        let call_count = Arc::new(Mutex::new(0));
        (
            Self {
                name,
                call_count: call_count.clone(),
            },
            call_count,
        )
    }

    fn sample_bars(ticker: &Ticker, range: DateRange) -> Vec<Ohlcv> {
        vec![
            Ohlcv::try_new(
                ticker.clone(),
                range.start,
                Decimal::from(100),
                Decimal::from(110),
                Decimal::from(90),
                Decimal::from(105),
                1_000,
            )
            .unwrap(),
            Ohlcv::try_new(
                ticker.clone(),
                range.end,
                Decimal::from(105),
                Decimal::from(115),
                Decimal::from(95),
                Decimal::from(110),
                2_000,
            )
            .unwrap(),
        ]
    }
}

#[async_trait]
impl MarketDataProvider for FakeProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn fetch(
        &self,
        ticker: &Ticker,
        range: DateRange,
        _interval: BarInterval,
    ) -> Result<Vec<Ohlcv>> {
        *self.call_count.lock().await += 1;
        Ok(Self::sample_bars(ticker, range))
    }
}

fn test_ticker() -> Ticker {
    Ticker::new("AAPL").with_exchange("NASDAQ")
}

fn test_range() -> DateRange {
    DateRange::try_new(
        NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn cache_stores_and_returns_series() {
    let cache = InMemoryCache::default();
    let ticker = test_ticker();
    let bar = Ohlcv::try_new(
        ticker.clone(),
        NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        Decimal::from(100),
        Decimal::from(110),
        Decimal::from(90),
        Decimal::from(105),
        1_000,
    )
    .unwrap();

    assert!(cache.get("key").await.is_none());
    cache.put("key", vec![bar.clone()]).await;
    let cached = cache.get("key").await.unwrap();
    assert_eq!(cached.len(), 1);
    assert_eq!(cached[0], bar);
}

#[tokio::test]
async fn moka_cache_stores_and_expires_entries() {
    let cache = MokaMarketDataCache::with_ttl(Duration::from_millis(100));
    let ticker = test_ticker();
    let bar = Ohlcv::try_new(
        ticker.clone(),
        NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        Decimal::from(100),
        Decimal::from(110),
        Decimal::from(90),
        Decimal::from(105),
        1_000,
    )
    .unwrap();

    cache.put("key", vec![bar.clone()]).await;
    let cached = cache.get("key").await.unwrap();
    assert_eq!(cached.len(), 1);
    assert_eq!(cached[0], bar);

    tokio::time::sleep(Duration::from_millis(250)).await;
    assert!(cache.get("key").await.is_none());
}

#[tokio::test]
async fn service_uses_default_provider_and_caches_result() {
    let cache: Arc<dyn MarketDataCache> = Arc::new(InMemoryCache::default());
    let (provider, call_count) = FakeProvider::new("fake");

    let service = MarketDataService::new("fake", cache);
    service.register_provider(Arc::new(provider));

    let ticker = test_ticker();
    let range = test_range();

    // First fetch should hit the provider and populate the cache.
    let first = service
        .fetch(&ticker, range, BarInterval::Daily, None)
        .await
        .unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(*call_count.lock().await, 1);

    // Second fetch with the same parameters should be a cache hit.
    let second = service
        .fetch(&ticker, range, BarInterval::Daily, None)
        .await
        .unwrap();
    assert_eq!(second, first);
    assert_eq!(*call_count.lock().await, 1);
}

#[tokio::test]
async fn service_resolves_named_provider() {
    let cache: Arc<dyn MarketDataCache> = Arc::new(InMemoryCache::default());
    let (default_provider, default_count) = FakeProvider::new("default_fake");
    let (named_provider, named_count) = FakeProvider::new("named_fake");

    let service = MarketDataService::new("default_fake", cache);
    service.register_provider(Arc::new(default_provider));
    service.register_provider(Arc::new(named_provider));

    let ticker = test_ticker();
    let range = test_range();

    service
        .fetch(&ticker, range, BarInterval::Daily, Some("named_fake"))
        .await
        .unwrap();

    assert_eq!(*default_count.lock().await, 0);
    assert_eq!(*named_count.lock().await, 1);
}

#[tokio::test]
async fn service_registration_is_visible_across_clones() {
    let cache: Arc<dyn MarketDataCache> = Arc::new(InMemoryCache::default());
    let (provider, call_count) = FakeProvider::new("fake");

    let service = MarketDataService::new("fake", cache);
    service.register_provider(Arc::new(provider));

    let cloned = service.clone();
    let bars = cloned
        .fetch(&test_ticker(), test_range(), BarInterval::Daily, None)
        .await
        .unwrap();
    assert_eq!(bars.len(), 2);
    assert_eq!(*call_count.lock().await, 1);
}

#[tokio::test]
async fn service_returns_error_when_provider_not_found() {
    let cache: Arc<dyn MarketDataCache> = Arc::new(InMemoryCache::default());
    let service = MarketDataService::new("missing", cache);

    let err = service
        .fetch(&test_ticker(), test_range(), BarInterval::Daily, None)
        .await
        .unwrap_err();

    match err {
        QuantError::ProviderNotAvailable(msg) => {
            assert!(msg.contains("missing"));
        }
        other => panic!("expected ProviderNotAvailable, got {other:?}"),
    }
}

#[tokio::test]
async fn yahoo_finance_provider_parses_stubbed_csv() {
    let mock_server = MockServer::start().await;

    let csv_body = "Date,Open,High,Low,Close,Adj Close,Volume\n\
                    2024-01-01,150.00,155.00,149.00,154.00,154.00,1000\n\
                    2024-01-02,155.00,156.00,154.00,155.50,155.50,2000\n\
                    2024-01-03,156.00,157.00,155.00,156.50,156.50,3000\n";

    Mock::given(method("GET"))
        .and(path("/v7/finance/download/AAPL"))
        .and(query_param("interval", "1d"))
        .respond_with(ResponseTemplate::new(200).set_body_string(csv_body))
        .mount(&mock_server)
        .await;

    let provider =
        YahooFinanceProvider::with_base_url(format!("{}/v7/finance/download", mock_server.uri()))
            .unwrap();
    let ticker = Ticker::new("AAPL");
    let range = DateRange::try_new(
        NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
    )
    .unwrap();

    let bars = provider
        .fetch(&ticker, range, BarInterval::Daily)
        .await
        .unwrap();

    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].date, NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
    assert_eq!(bars[0].open, Decimal::try_from(155.0).unwrap());
    assert_eq!(bars[1].close, Decimal::try_from(156.5).unwrap());
}

#[tokio::test]
async fn yahoo_finance_provider_maps_http_errors_to_provider_not_available() {
    for status in [404u16, 500] {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v7/finance/download/AAPL"))
            .and(query_param("interval", "1d"))
            .respond_with(ResponseTemplate::new(status))
            .mount(&mock_server)
            .await;

        let provider = YahooFinanceProvider::with_base_url(format!(
            "{}/v7/finance/download",
            mock_server.uri()
        ))
        .unwrap();

        let err = provider
            .fetch(&Ticker::new("AAPL"), test_range(), BarInterval::Daily)
            .await
            .unwrap_err();

        match err {
            QuantError::ProviderNotAvailable(msg) => {
                assert!(msg.contains("HTTP") || msg.contains("404") || msg.contains("500"));
            }
            other => panic!("status {status} expected ProviderNotAvailable, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn yahoo_finance_provider_rejects_empty_base_url() {
    let err = YahooFinanceProvider::with_base_url("").unwrap_err();
    assert!(matches!(err, QuantError::InvalidCommand(_)));
    assert!(err.to_string().contains("base URL must be non-empty"));
}
