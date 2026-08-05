//! Integration tests for the `sqt-orders` crate.

use std::sync::Arc;

use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use sqt_orders::{InMemoryOrderRepository, OrderService, OrderSide, OrderStatus, OrderType};

fn build_service() -> OrderService<InMemoryOrderRepository> {
    OrderService::new(Arc::new(InMemoryOrderRepository::new()))
}

#[tokio::test]
async fn create_and_find_order() {
    let service = build_service();
    let order = service
        .create_order(
            "AAPL",
            OrderSide::Buy,
            OrderType::Market,
            Decimal::from(100),
            None,
        )
        .await
        .expect("create succeeds");

    assert_eq!(order.ticker, "AAPL");
    assert_eq!(order.status, OrderStatus::Created);

    let found = service
        .get_order(order.id)
        .await
        .expect("find succeeds")
        .expect("order exists");
    assert_eq!(found.id, order.id);
}

#[tokio::test]
async fn order_lifecycle_transitions() {
    let service = build_service();
    let order = service
        .create_order(
            "MSFT",
            OrderSide::Sell,
            OrderType::Limit,
            Decimal::from(50),
            Some(Decimal::from_f64(300.0).unwrap()),
        )
        .await
        .expect("create succeeds");

    let submitted = service
        .submit_order(order.id)
        .await
        .expect("submit succeeds");
    assert_eq!(submitted.status, OrderStatus::Submitted);

    let filled = service.fill_order(order.id).await.expect("fill succeeds");
    assert_eq!(filled.status, OrderStatus::Filled);
}

#[tokio::test]
async fn invalid_transition_fails() {
    let service = build_service();
    let order = service
        .create_order(
            "TSLA",
            OrderSide::Buy,
            OrderType::Market,
            Decimal::from(10),
            None,
        )
        .await
        .expect("create succeeds");

    service
        .fill_order(order.id)
        .await
        .expect_err("cannot fill created order");
}

#[tokio::test]
async fn cancel_order() {
    let service = build_service();
    let order = service
        .create_order(
            "GOOGL",
            OrderSide::Buy,
            OrderType::Market,
            Decimal::from(20),
            None,
        )
        .await
        .expect("create succeeds");

    let cancelled = service
        .cancel_order(order.id)
        .await
        .expect("cancel succeeds");
    assert_eq!(cancelled.status, OrderStatus::Cancelled);
}

#[tokio::test]
async fn list_orders() {
    let service = build_service();
    service
        .create_order(
            "AAPL",
            OrderSide::Buy,
            OrderType::Market,
            Decimal::from(10),
            None,
        )
        .await
        .unwrap();
    service
        .create_order(
            "MSFT",
            OrderSide::Sell,
            OrderType::Market,
            Decimal::from(20),
            None,
        )
        .await
        .unwrap();

    let orders = service.list_orders().await.expect("list succeeds");
    assert_eq!(orders.len(), 2);
}

#[tokio::test]
async fn limit_order_requires_price() {
    let service = build_service();
    let err = service
        .create_order(
            "AAPL",
            OrderSide::Buy,
            OrderType::Limit,
            Decimal::from(10),
            None,
        )
        .await
        .expect_err("limit order without price should fail");

    assert!(matches!(err, sqt_core::QuantError::InvalidCommand(_)));
}
