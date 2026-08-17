//! Service wiring helpers.

use std::sync::Arc;

use sqt_agent::ToolDispatcher;
use sqt_analysis::AnalysisService;
use sqt_backtest::BacktestService;
use sqt_marketdata::{MarketDataService, MokaMarketDataCache, YahooFinanceProvider};
use sqt_orders::{InMemoryOrderRepository, OrderRepository, SqlxOrderRepository};
use sqt_portfolio::PortfolioService;
use sqt_screener::{HardcodedFundamentalProvider, ScreenerService};
use tracing::{info, warn};

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

/// Builds the shared order repository.
///
/// When `DATABASE_URL` is set, a PostgreSQL-backed repository is created and
/// its schema is migrated. Otherwise an in-memory repository is used and a
/// warning is logged because durability is disabled.
pub async fn build_order_repository() -> Arc<dyn OrderRepository> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => {
            info!("using PostgreSQL order repository");
            match sqlx::PgPool::connect(&url).await {
                Ok(pool) => {
                    let repo = SqlxOrderRepository::new(pool);
                    if let Err(e) = repo.migrate().await {
                        warn!(error = %e, "failed to migrate orders table; falling back to in-memory repository");
                        return Arc::new(InMemoryOrderRepository::new());
                    }
                    Arc::new(repo)
                }
                Err(e) => {
                    warn!(error = %e, "failed to connect to DATABASE_URL; falling back to in-memory order repository");
                    Arc::new(InMemoryOrderRepository::new())
                }
            }
        }
        Err(_) => {
            warn!("DATABASE_URL not set; using in-memory order repository (durability disabled)");
            Arc::new(InMemoryOrderRepository::new())
        }
    }
}
