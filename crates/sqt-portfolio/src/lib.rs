//! Portfolio construction and optimization for the Standard Tools Rust port.
//!
//! This crate provides a small collection of portfolio optimizers that operate
//! on historical return series. All optimizers return asset weights as
//! `HashMap<String, f64>` and expose both low-level functions and a
//! convenient [`PortfolioService`] façade that accepts maps of returns.
//!
//! # Available optimizers
//!
//! * [`mean_variance`] — mean-variance optimization via a two-fund separation
//!   heuristic (minimum-variance + maximum-Sharpe portfolios).
//! * [`risk_parity`] — inverse-volatility risk parity.
//! * [`black_litterman`] — Black-Litterman model with explicit view matrices or
//!   a simplified expert-view interface.

pub mod black_litterman;
pub mod mean_variance;
pub mod risk_parity;
pub mod service;

pub(crate) mod validation;

pub use black_litterman::{
    optimize as black_litterman_optimize,
    optimize_simplified as black_litterman_optimize_simplified, BlackLittermanResult,
};
pub use mean_variance::{optimize as mean_variance_optimize, MeanVarianceResult};
pub use risk_parity::{optimize as risk_parity_optimize, RiskParityResult};
pub use service::PortfolioService;
