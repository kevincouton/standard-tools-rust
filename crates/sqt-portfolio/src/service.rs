//! High-level portfolio service that operates on maps of return series.
//!
//! The service is a thin façade over the low-level optimizers. It takes care of
//! extracting sorted labels and aligning the returns matrix so that callers can
//! work with ergonomic `HashMap<String, Vec<f64>>` inputs.

use std::collections::HashMap;

use sqt_core::{QuantError, Result};

use crate::black_litterman::BlackLittermanResult;
use crate::mean_variance::{optimize as mean_variance_optimize, MeanVarianceResult};
use crate::risk_parity::{optimize as risk_parity_optimize, RiskParityResult};

/// High-level entry point for portfolio optimization on labelled return series.
#[derive(Debug, Clone, Default)]
pub struct PortfolioService;

impl PortfolioService {
    /// Creates a new portfolio service.
    pub fn new() -> Self {
        Self
    }

    /// Runs mean-variance optimization on the supplied returns map.
    ///
    /// See [`crate::mean_variance::optimize`] for details on the optimisation
    /// method and the meaning of `risk_free_rate` and `target_return`.
    ///
    /// # Errors
    ///
    /// Returns [`QuantError::DataQuality`] if the returns map is empty or if
    /// any series is too short. Returns the underlying optimizer errors for
    /// numerical failures.
    pub fn mean_variance(
        &self,
        returns: &HashMap<String, Vec<f64>>,
        risk_free_rate: f64,
        target_return: Option<f64>,
    ) -> Result<MeanVarianceResult> {
        let (labels, matrix) = to_matrix(returns)?;
        mean_variance_optimize(&matrix, &labels, risk_free_rate, target_return)
    }

    /// Computes inverse-volatility risk-parity weights on the supplied returns
    /// map.
    ///
    /// # Errors
    ///
    /// Returns [`QuantError::DataQuality`] if the returns map is empty or if
    /// any series is too short.
    pub fn risk_parity(&self, returns: &HashMap<String, Vec<f64>>) -> Result<RiskParityResult> {
        let (labels, matrix) = to_matrix(returns)?;
        risk_parity_optimize(&matrix, &labels)
    }

    /// Runs the Black-Litterman model using independent expert views keyed by
    /// ticker symbol.
    ///
    /// `market_caps` and `views` must contain an entry for every ticker in
    /// `returns`. See [`crate::black_litterman::optimize_simplified`] for the
    /// model details.
    ///
    /// # Errors
    ///
    /// Returns [`QuantError::InvalidCommand`] if any required ticker is missing
    /// from `market_caps` or `views`, or if model parameters are invalid.
    pub fn black_litterman(
        &self,
        returns: &HashMap<String, Vec<f64>>,
        market_caps: &HashMap<String, f64>,
        views: &HashMap<String, f64>,
        tau: f64,
        risk_aversion: f64,
    ) -> Result<BlackLittermanResult> {
        let (labels, matrix) = to_matrix(returns)?;
        crate::black_litterman::optimize_simplified(
            &labels,
            &matrix,
            market_caps,
            views,
            tau,
            risk_aversion,
        )
    }
}

fn to_matrix(returns: &HashMap<String, Vec<f64>>) -> Result<(Vec<String>, Vec<Vec<f64>>)> {
    if returns.is_empty() {
        return Err(QuantError::DataQuality(
            "returns map must contain at least one series".to_string(),
        ));
    }
    let mut labels: Vec<String> = returns.keys().cloned().collect();
    labels.sort();
    let matrix: Vec<Vec<f64>> = labels
        .iter()
        .map(|l| returns.get(l).expect("key exists").clone())
        .collect();
    Ok((labels, matrix))
}
