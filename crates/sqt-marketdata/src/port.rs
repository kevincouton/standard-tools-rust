//! Port traits that define the boundary between market-data consumers and
//! concrete data sources or caches.
//!
//! The traits are object-safe and use [`async_trait`] so they can be stored as
//! `Arc<dyn MarketDataProvider>` and `Arc<dyn MarketDataCache>` inside the
//! [`MarketDataService`](crate::service::MarketDataService).

use async_trait::async_trait;
use sqt_core::{BarInterval, DateRange, Ohlcv, Result, Ticker};

/// A market-data provider fetches historical OHLCV series for a ticker.
#[async_trait]
pub trait MarketDataProvider: Send + Sync {
    /// Returns the provider's unique name, e.g. `"yahoo_finance"`.
    fn name(&self) -> &'static str;

    /// Fetches a series of OHLCV bars for `ticker` over `range` at `interval`.
    async fn fetch(
        &self,
        ticker: &Ticker,
        range: DateRange,
        interval: BarInterval,
    ) -> Result<Vec<Ohlcv>>;
}

/// A cache for market-data series keyed by arbitrary strings.
#[async_trait]
pub trait MarketDataCache: Send + Sync {
    /// Returns the cached series for `key`, if any.
    async fn get(&self, key: &str) -> Option<Vec<Ohlcv>>;

    /// Stores `series` under `key`.
    async fn put(&self, key: &str, series: Vec<Ohlcv>);
}
