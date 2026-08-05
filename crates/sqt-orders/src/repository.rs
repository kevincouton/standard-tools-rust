//! Order repository.

use async_trait::async_trait;
use sqlx::{PgPool, Row};
use sqt_core::{QuantError, Result};
use uuid::Uuid;

use crate::domain::Order;

/// Abstract persistence for orders.
#[async_trait]
pub trait OrderRepository: Send + Sync {
    /// Saves an order.
    async fn save(&self, order: &Order) -> Result<()>;

    /// Finds an order by id.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Order>>;

    /// Lists all orders.
    async fn list(&self) -> Result<Vec<Order>>;

    /// Deletes an order by id.
    async fn delete(&self, id: Uuid) -> Result<()>;
}

/// In-memory repository for tests and local development.
#[derive(Default, Debug, Clone)]
pub struct InMemoryOrderRepository {
    orders: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<Uuid, Order>>>,
}

impl InMemoryOrderRepository {
    /// Creates a new empty repository.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl OrderRepository for InMemoryOrderRepository {
    async fn save(&self, order: &Order) -> Result<()> {
        let mut orders = self.orders.lock().await;
        orders.insert(order.id, order.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Order>> {
        let orders = self.orders.lock().await;
        Ok(orders.get(&id).cloned())
    }

    async fn list(&self) -> Result<Vec<Order>> {
        let orders = self.orders.lock().await;
        Ok(orders.values().cloned().collect())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let mut orders = self.orders.lock().await;
        orders.remove(&id);
        Ok(())
    }
}

/// PostgreSQL-backed order repository.
#[derive(Debug, Clone)]
pub struct SqlxOrderRepository {
    pool: PgPool,
}

impl SqlxOrderRepository {
    /// Creates a new repository from an existing pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creates the orders table if it does not already exist.
    pub async fn migrate(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS orders (
                id UUID PRIMARY KEY,
                client_order_id TEXT,
                ticker TEXT NOT NULL,
                side TEXT NOT NULL,
                order_type TEXT NOT NULL,
                quantity NUMERIC NOT NULL,
                price NUMERIC,
                status TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| QuantError::Internal(anyhow::Error::new(e)))?;
        Ok(())
    }
}

#[async_trait]
impl OrderRepository for SqlxOrderRepository {
    async fn save(&self, order: &Order) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO orders
                (id, client_order_id, ticker, side, order_type, quantity, price, status, created_at, updated_at)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                client_order_id = EXCLUDED.client_order_id,
                ticker = EXCLUDED.ticker,
                side = EXCLUDED.side,
                order_type = EXCLUDED.order_type,
                quantity = EXCLUDED.quantity,
                price = EXCLUDED.price,
                status = EXCLUDED.status,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(order.id)
        .bind(&order.client_order_id)
        .bind(&order.ticker)
        .bind(order.side.as_str())
        .bind(order.order_type.as_str())
        .bind(order.quantity)
        .bind(order.price)
        .bind(order.status.as_str())
        .bind(order.created_at)
        .bind(order.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| QuantError::Internal(anyhow::Error::new(e)))?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Order>> {
        let row = sqlx::query("SELECT * FROM orders WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| QuantError::Internal(anyhow::Error::new(e)))?;
        Ok(row.map(|r| map_row(&r)).transpose()?)
    }

    async fn list(&self) -> Result<Vec<Order>> {
        let rows = sqlx::query("SELECT * FROM orders ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| QuantError::Internal(anyhow::Error::new(e)))?;
        rows.into_iter().map(|r| map_row(&r)).collect()
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM orders WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| QuantError::Internal(anyhow::Error::new(e)))?;
        Ok(())
    }
}

fn map_row(row: &sqlx::postgres::PgRow) -> Result<Order> {
    Ok(Order {
        id: row.try_get("id").map_err(sql_err)?,
        client_order_id: row.try_get("client_order_id").map_err(sql_err)?,
        ticker: row.try_get("ticker").map_err(sql_err)?,
        side: parse_enum(row.try_get::<String, _>("side").map_err(sql_err)?)?,
        order_type: parse_enum(row.try_get::<String, _>("order_type").map_err(sql_err)?)?,
        quantity: row.try_get("quantity").map_err(sql_err)?,
        price: row.try_get("price").map_err(sql_err)?,
        status: parse_enum(row.try_get::<String, _>("status").map_err(sql_err)?)?,
        created_at: row.try_get("created_at").map_err(sql_err)?,
        updated_at: row.try_get("updated_at").map_err(sql_err)?,
    })
}

fn sql_err(e: sqlx::Error) -> QuantError {
    QuantError::Internal(anyhow::Error::new(e))
}

fn parse_enum<E: std::str::FromStr>(value: String) -> Result<E> {
    value
        .parse()
        .map_err(|_| QuantError::DataQuality(format!("invalid enum value `{value}`")))
}

impl std::str::FromStr for crate::domain::OrderSide {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "buy" => Ok(Self::Buy),
            "sell" => Ok(Self::Sell),
            _ => Err(()),
        }
    }
}

impl std::str::FromStr for crate::domain::OrderType {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "market" => Ok(Self::Market),
            "limit" => Ok(Self::Limit),
            "stop" => Ok(Self::Stop),
            _ => Err(()),
        }
    }
}

impl std::str::FromStr for crate::domain::OrderStatus {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "created" => Ok(Self::Created),
            "submitted" => Ok(Self::Submitted),
            "filled" => Ok(Self::Filled),
            "partially_filled" => Ok(Self::PartiallyFilled),
            "cancelled" => Ok(Self::Cancelled),
            "rejected" => Ok(Self::Rejected),
            _ => Err(()),
        }
    }
}
