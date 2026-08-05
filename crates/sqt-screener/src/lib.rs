//! Fundamental and technical stock screener for the Standard Tools Rust port.
//!
//! This crate provides data types for fundamental metrics, a pluggable provider
//! trait, and a [`ScreenerService`] that applies both fundamental and
//! technical-indicator filters to a universe of securities.

pub mod fundamental;
pub mod provider;
pub mod service;

pub use fundamental::{FundamentalData, FundamentalFilter};
pub use provider::{FundamentalProvider, HardcodedFundamentalProvider};
pub use service::{Comparator, IndicatorFilter, ScreenerService};
