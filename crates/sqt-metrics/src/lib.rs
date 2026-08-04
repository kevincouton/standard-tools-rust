//! Risk and return metrics crate for the Standard Tools Rust port.
//!
//! Provides [`MetricsCalculator`], which computes a standard set of portfolio
//! performance metrics from a vector of period returns. Calculations use
//! `ndarray` for vector operations and `statrs` for statistical summaries.

pub mod calculator;

pub use calculator::{MetricsCalculator, MetricsResult};
