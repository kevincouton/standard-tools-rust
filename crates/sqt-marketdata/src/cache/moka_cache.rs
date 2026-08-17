//! Moka-backed asynchronous market-data cache.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use moka::future::Cache as MokaCache;
use sqt_core::Ohlcv;

use crate::port::MarketDataCache;

/// Default TTL for cached series.
pub const DEFAULT_TTL: Duration = Duration::from_secs(5 * 60);

pub const DEFAULT_MAX_CAPACITY: u64 = 10_000;

/// A [`MarketDataCache`] implementation backed by [`moka::future::Cache`].
///
/// Entries are stored with a configurable time-to-live. After the TTL expires,
/// the entry is evicted on the next access.
#[derive(Clone)]
pub struct MokaMarketDataCache {
    inner: MokaCache<String, Vec<Ohlcv>>,
}

impl MokaMarketDataCache {
    /// Creates a new cache with the default TTL ([`DEFAULT_TTL`]) and default
    /// max capacity ([`DEFAULT_MAX_CAPACITY`]).
    pub fn new() -> Self {
        Self::with_ttl_and_capacity(DEFAULT_TTL, DEFAULT_MAX_CAPACITY)
    }

    /// Creates a new cache with the given `ttl` and default max capacity.
    pub fn with_ttl(ttl: Duration) -> Self {
        Self::with_ttl_and_capacity(ttl, DEFAULT_MAX_CAPACITY)
    }

    /// Creates a new cache with the given `ttl` and `max_capacity`.
    pub fn with_ttl_and_capacity(ttl: Duration, max_capacity: u64) -> Self {
        Self {
            inner: MokaCache::builder()
                .max_capacity(max_capacity)
                .time_to_live(ttl)
                .build(),
        }
    }
}

impl Default for MokaMarketDataCache {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketDataCache for MokaMarketDataCache {
    async fn get(&self, key: &str) -> Option<Vec<Ohlcv>> {
        self.inner.get(key).await
    }

    async fn put(&self, key: &str, series: Vec<Ohlcv>) {
        self.inner.insert(key.to_string(), series).await;
    }
}

// Moka caches are best shared through cloning because the underlying cache is
// already reference-counted. This alias makes it easy to wrap the cache in an
// `Arc<dyn MarketDataCache>` for the service.
impl From<MokaMarketDataCache> for Arc<dyn MarketDataCache> {
    fn from(cache: MokaMarketDataCache) -> Self {
        Arc::new(cache)
    }
}
