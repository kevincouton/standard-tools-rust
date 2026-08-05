//! Order management crate for Standard-Tools.
//!
//! Provides a domain model, in-memory and SQLx repositories, and an
//! application service for creating and lifecycle-managing trade orders.

pub mod domain;
pub mod repository;
pub mod service;

pub use domain::{Order, OrderSide, OrderStatus, OrderType};
pub use repository::{InMemoryOrderRepository, OrderRepository, SqlxOrderRepository};
pub use service::OrderService;

/// Convenience `Result` alias used by this crate.
pub type Result<T> = sqt_core::Result<T>;
