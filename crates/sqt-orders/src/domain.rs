//! Order domain model.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqt_core::{QuantError, Result};
use uuid::Uuid;

/// Side of an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderSide {
    Buy,
    Sell,
}

impl OrderSide {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderSide::Buy => "buy",
            OrderSide::Sell => "sell",
        }
    }
}

/// Order type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    Market,
    Limit,
    Stop,
}

impl OrderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderType::Market => "market",
            OrderType::Limit => "limit",
            OrderType::Stop => "stop",
        }
    }
}

/// Lifecycle status of an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Created,
    Submitted,
    Filled,
    PartiallyFilled,
    Cancelled,
    Rejected,
}

impl OrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderStatus::Created => "created",
            OrderStatus::Submitted => "submitted",
            OrderStatus::Filled => "filled",
            OrderStatus::PartiallyFilled => "partially_filled",
            OrderStatus::Cancelled => "cancelled",
            OrderStatus::Rejected => "rejected",
        }
    }
}

/// A trade order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Order {
    /// Unique order identifier.
    pub id: Uuid,
    /// Human-readable order identifier (optional).
    pub client_order_id: Option<String>,
    /// Ticker to trade.
    pub ticker: String,
    /// Buy or sell.
    pub side: OrderSide,
    /// Market, limit, or stop.
    pub order_type: OrderType,
    /// Number of shares/contracts.
    pub quantity: Decimal,
    /// Limit or stop price (required for non-market orders).
    pub price: Option<Decimal>,
    /// Current status.
    pub status: OrderStatus,
    /// UTC timestamp when the order was created.
    pub created_at: DateTime<Utc>,
    /// UTC timestamp of the last status update.
    pub updated_at: DateTime<Utc>,
}

impl Order {
    /// Creates a new order in `Created` status.
    pub fn new(
        ticker: impl Into<String>,
        side: OrderSide,
        order_type: OrderType,
        quantity: Decimal,
        price: Option<Decimal>,
    ) -> Result<Self> {
        if quantity <= Decimal::ZERO {
            return Err(QuantError::InvalidCommand(
                "quantity must be positive".into(),
            ));
        }

        if matches!(order_type, OrderType::Limit | OrderType::Stop) && price.is_none() {
            return Err(QuantError::InvalidCommand(
                "limit and stop orders require a price".into(),
            ));
        }

        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            client_order_id: None,
            ticker: ticker.into(),
            side,
            order_type,
            quantity,
            price,
            status: OrderStatus::Created,
            created_at: now,
            updated_at: now,
        })
    }

    /// Submits the order.
    pub fn submit(&mut self) -> Result<()> {
        self.transition_to(OrderStatus::Submitted)
    }

    /// Fills the order fully.
    pub fn fill(&mut self) -> Result<()> {
        self.transition_to(OrderStatus::Filled)
    }

    /// Fills the order partially.
    pub fn partial_fill(&mut self) -> Result<()> {
        self.transition_to(OrderStatus::PartiallyFilled)
    }

    /// Cancels the order.
    pub fn cancel(&mut self) -> Result<()> {
        self.transition_to(OrderStatus::Cancelled)
    }

    /// Rejects the order.
    pub fn reject(&mut self) -> Result<()> {
        self.transition_to(OrderStatus::Rejected)
    }

    fn transition_to(&mut self, new_status: OrderStatus) -> Result<()> {
        use OrderStatus::*;
        let allowed = match self.status {
            Created => vec![Submitted, Cancelled, Rejected],
            Submitted => vec![Filled, PartiallyFilled, Cancelled, Rejected],
            PartiallyFilled => vec![Filled, Cancelled, Rejected],
            Filled | Cancelled | Rejected => vec![],
        };

        if !allowed.contains(&new_status) {
            return Err(QuantError::InvalidCommand(format!(
                "cannot transition from {:?} to {:?}",
                self.status, new_status
            )));
        }

        self.status = new_status;
        self.updated_at = Utc::now();
        Ok(())
    }
}
