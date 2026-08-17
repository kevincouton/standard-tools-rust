//! Shared application state.

use std::sync::Arc;

use sqt_agent::ToolDispatcher;
use sqt_audit::{AuditStorage, AuditWriter};
use sqt_marketdata::MarketDataService;
use sqt_orders::OrderRepository;
use sqt_screener::HardcodedFundamentalProvider;

/// Application state shared by all HTTP/gRPC handlers.
#[derive(Clone)]
pub struct AppState<S: AuditStorage> {
    /// Agent tool dispatcher.
    pub dispatcher: Arc<ToolDispatcher<HardcodedFundamentalProvider>>,
    /// Audit writer.
    pub audit_writer: Arc<AuditWriter<S>>,
    /// Market data service (also reachable through the dispatcher).
    pub market_data: Arc<MarketDataService>,
    /// Order repository (shared across requests for durability).
    pub order_repo: Arc<dyn OrderRepository>,
}
