//! Order application service.

use std::sync::Arc;

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::{Order, OrderSide, OrderStatus, OrderType};
use crate::repository::OrderRepository;

/// Application service for order management.
#[derive(Clone)]
pub struct OrderService<R: OrderRepository + ?Sized> {
    repo: Arc<R>,
}

impl<R: OrderRepository + ?Sized> OrderService<R> {
    /// Creates a service backed by the supplied repository.
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    /// Creates and persists a new order.
    pub async fn create_order(
        &self,
        ticker: impl Into<String>,
        side: OrderSide,
        order_type: OrderType,
        quantity: Decimal,
        price: Option<Decimal>,
    ) -> crate::Result<Order> {
        let order = Order::new(ticker, side, order_type, quantity, price)?;
        self.repo.save(&order).await?;
        Ok(order)
    }

    /// Submits an existing order.
    pub async fn submit_order(&self, id: Uuid) -> crate::Result<Order> {
        let mut order = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| sqt_core::QuantError::NotFound(format!("order {id} not found")))?;
        order.submit()?;
        self.repo.save(&order).await?;
        Ok(order)
    }

    /// Fills an existing order.
    pub async fn fill_order(&self, id: Uuid) -> crate::Result<Order> {
        let mut order = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| sqt_core::QuantError::NotFound(format!("order {id} not found")))?;
        order.fill()?;
        self.repo.save(&order).await?;
        Ok(order)
    }

    /// Cancels an existing order.
    pub async fn cancel_order(&self, id: Uuid) -> crate::Result<Order> {
        let mut order = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| sqt_core::QuantError::NotFound(format!("order {id} not found")))?;
        order.cancel()?;
        self.repo.save(&order).await?;
        Ok(order)
    }

    /// Finds an order by id.
    pub async fn get_order(&self, id: Uuid) -> crate::Result<Option<Order>> {
        self.repo.find_by_id(id).await
    }

    /// Lists all orders.
    pub async fn list_orders(&self) -> crate::Result<Vec<Order>> {
        self.repo.list().await
    }

    /// Returns the number of orders.
    pub async fn count_orders(&self) -> crate::Result<usize> {
        let orders = self.repo.list().await?;
        Ok(orders.len())
    }

    /// Returns orders filtered by status.
    pub async fn orders_by_status(&self, status: OrderStatus) -> crate::Result<Vec<Order>> {
        let orders = self.repo.list().await?;
        Ok(orders.into_iter().filter(|o| o.status == status).collect())
    }

    /// Deletes an order by id.
    pub async fn delete_order(&self, id: Uuid) -> crate::Result<()> {
        self.repo.delete(id).await
    }
}
