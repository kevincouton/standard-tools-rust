//! Market-data fetching, caching, and service orchestration.
//!
//! This crate provides:
//!
//! * [`port::MarketDataProvider`] and [`port::MarketDataCache`] traits that
//!   define the integration boundaries.
//! * [`service::MarketDataService`], which resolves providers and caches results.
//! * [`providers::yfinance::YahooFinanceProvider`], a Yahoo Finance adapter.
//! * [`cache::moka_cache::MokaMarketDataCache`], an asynchronous Moka-backed cache.

pub mod cache;
pub mod port;
pub mod providers;
pub mod service;

pub use cache::moka_cache::MokaMarketDataCache;
pub use port::{MarketDataCache, MarketDataProvider};
pub use providers::yfinance::YahooFinanceProvider;
pub use service::MarketDataService;
