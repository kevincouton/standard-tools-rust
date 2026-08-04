//! Market-data service that orchestrates providers and caching.

use std::sync::Arc;

use dashmap::DashMap;
use sqt_core::{BarInterval, DateRange, Ohlcv, QuantError, Result, Ticker};
use tracing::{debug, instrument};

use crate::port::{MarketDataCache, MarketDataProvider};

/// Orchestrates market-data fetching across registered providers with caching.
///
/// The service resolves a provider name (explicit or default), checks the cache,
/// and falls back to the provider on a miss. Providers and caches are stored as
/// trait objects so consumers can mix and match implementations.
///
/// Cloning the service is cheap and shares the same provider registry, so a
/// provider registered on one clone is visible to all other clones.
#[derive(Clone)]
pub struct MarketDataService {
    default_provider: String,
    providers: Arc<DashMap<String, Arc<dyn MarketDataProvider>>>,
    cache: Arc<dyn MarketDataCache>,
}

impl MarketDataService {
    /// Creates a new service with the given default provider name and cache.
    pub fn new(default_provider: impl Into<String>, cache: Arc<dyn MarketDataCache>) -> Self {
        Self {
            default_provider: default_provider.into(),
            providers: Arc::new(DashMap::new()),
            cache,
        }
    }

    /// Registers a provider so it can be resolved by name in [`Self::fetch`].
    ///
    /// Registrations are visible to all clones of this service because the
    /// registry is stored behind an [`Arc`].
    pub fn register_provider(&self, provider: Arc<dyn MarketDataProvider>) {
        self.providers.insert(provider.name().to_string(), provider);
    }

    /// Fetches a series of OHLCV bars.
    ///
    /// # Arguments
    ///
    /// * `ticker` - Instrument to fetch.
    /// * `range` - Inclusive date range.
    /// * `interval` - Bar aggregation interval.
    /// * `provider` - Optional provider name; defaults to the service default.
    ///
    /// # Errors
    ///
    /// Returns [`QuantError::ProviderNotAvailable`] if the requested provider is
    /// not registered.
    #[instrument(skip(self), fields(%ticker.symbol, provider))]
    pub async fn fetch(
        &self,
        ticker: &Ticker,
        range: DateRange,
        interval: BarInterval,
        provider: Option<&str>,
    ) -> Result<Vec<Ohlcv>> {
        let provider_name = provider.unwrap_or(&self.default_provider);
        debug!(%provider_name, "resolved provider");

        let provider = self
            .providers
            .get(provider_name)
            .map(|r| r.clone())
            .ok_or_else(|| {
                QuantError::ProviderNotAvailable(format!(
                    "provider `{provider_name}` is not registered"
                ))
            })?;

        let cache_key = cache_key(provider_name, ticker, interval, range);

        if let Some(series) = self.cache.get(&cache_key).await {
            debug!(%cache_key, "cache hit");
            return Ok(series);
        }

        debug!(%cache_key, "cache miss; delegating to provider");
        let series = provider.fetch(ticker, range, interval).await?;
        self.cache.put(&cache_key, series.clone()).await;
        Ok(series)
    }
}

/// Builds a deterministic cache key for a fetch request.
fn cache_key(
    provider_name: &str,
    ticker: &Ticker,
    interval: BarInterval,
    range: DateRange,
) -> String {
    let exchange = ticker.exchange.as_deref().unwrap_or("");
    let interval_str = interval.to_string();
    format!(
        "{provider_name}:{symbol}:{exchange}:{interval_str}:{start}:{end}",
        symbol = ticker.symbol,
        start = range.start,
        end = range.end,
    )
}
