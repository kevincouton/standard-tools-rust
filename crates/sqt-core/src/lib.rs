//! Core shared types, errors, and value objects for the Standard Tools Rust port.
//!
//! This crate provides the foundational [`QuantError`] error type and reusable
//! domain values such as [`Ticker`], [`DateRange`], [`Ohlcv`], and [`BarInterval`].
//! These types are intentionally kept dependency-light and serializable so they
//! can be shared across workspace crates.

pub mod error;
pub mod value_objects;

pub use error::{QuantError, Result};
pub use value_objects::{BarInterval, DateRange, Ohlcv, Ticker};
