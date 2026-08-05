//! Quantitative analysis utilities for the Standard Tools Rust port.
//!
//! This crate provides regression, cointegration, Hurst exponent estimation,
//! principal component analysis, correlation, multi-factor regression, and
//! Black-Scholes option pricing. The [`AnalysisService`] type offers a
//! convenient façade that accepts [`sqt_core::Ohlcv`] bars and delegates to the
//! underlying calculators.

pub mod cointegration;
pub mod correlation;
pub mod hurst;
pub mod multi_factor;
pub mod options;
pub mod pca;
pub mod regression;
pub mod service;

pub(crate) mod math;

pub use cointegration::{cointegration, CointegrationResult};
pub use correlation::{correlation, CorrelationMatrix};
pub use hurst::{hurst_exponent, HurstResult};
pub use multi_factor::{multi_factor_regression, MultiFactorResult};
pub use options::{black_scholes, BlackScholesParams, OptionPricingResult, OptionType};
pub use pca::{pca, PcaResult};
pub use regression::{linear_regression, LinearRegressionResult};
pub use service::AnalysisService;
