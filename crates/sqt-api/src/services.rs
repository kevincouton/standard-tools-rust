//! Service wiring helpers.

use std::sync::Arc;

use sqt_agent::ToolDispatcher;
use sqt_analysis::AnalysisService;
use sqt_backtest::BacktestService;
use sqt_marketdata::{MarketDataService, MokaMarketDataCache, YahooFinanceProvider};
use sqt_portfolio::PortfolioService;
use sqt_screener::{HardcodedFundamentalProvider, ScreenerService};

/// Builds the default market-data service with an in-memory Moka cache and the
/// Yahoo Finance provider registered.
pub fn build_market_data_service() -> Arc<MarketDataService> {
    let cache: Arc<dyn sqt_marketdata::MarketDataCache> = MokaMarketDataCache::new().into();
    let service = MarketDataService::new("yahoo", cache);
    service.register_provider(Arc::new(YahooFinanceProvider::new()));
    Arc::new(service)
}

/// Builds a tool dispatcher backed by the default fundamental provider.
pub fn build_dispatcher(
    market_data: Arc<MarketDataService>,
) -> ToolDispatcher<HardcodedFundamentalProvider> {
    ToolDispatcher::new(
        market_data,
        BacktestService::new(),
        AnalysisService::new(),
        PortfolioService::new(),
        ScreenerService::new(HardcodedFundamentalProvider::new()),
    )
}
