//! High-level analysis service that operates on [`sqt_core::Ohlcv`] bars.
//!
//! The service hides the mechanical work of extracting close prices, aligning
//! series by date, and converting price levels to returns. All analysis
//! calculators live in the sibling modules and are invoked from here.

use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use rust_decimal::prelude::ToPrimitive;
use sqt_core::{Ohlcv, QuantError, Result};

use crate::cointegration::{cointegration, CointegrationResult};
use crate::correlation::{correlation, CorrelationMatrix};
use crate::hurst::{hurst_exponent, HurstResult};
use crate::multi_factor::{multi_factor_regression, MultiFactorResult};
use crate::options::{black_scholes, BlackScholesParams, OptionPricingResult};
use crate::pca::{pca, PcaResult};
use crate::regression::{linear_regression, LinearRegressionResult};

/// High-level entry point for quantitative analysis on OHLCV data.
#[derive(Debug, Clone, Default)]
pub struct AnalysisService;

impl AnalysisService {
    /// Creates a new analysis service.
    pub fn new() -> Self {
        Self
    }

    /// Runs a single-variable linear regression of `asset` on `benchmark`.
    ///
    /// Both series are aligned by date, simple returns are computed, and the
    /// supplied `risk_free_rate` is subtracted from each period return before
    /// regression (treat `risk_free_rate` as the same periodic rate as the
    /// bar interval).
    pub fn regression(
        &self,
        asset: &[Ohlcv],
        benchmark: &[Ohlcv],
        risk_free_rate: f64,
    ) -> Result<LinearRegressionResult> {
        let (asset_closes, benchmark_closes) = align_two(asset, benchmark)?;
        let asset_returns = to_returns(&asset_closes)
            .into_iter()
            .map(|r| r - risk_free_rate)
            .collect::<Vec<_>>();
        let benchmark_returns = to_returns(&benchmark_closes)
            .into_iter()
            .map(|r| r - risk_free_rate)
            .collect::<Vec<_>>();
        linear_regression(&asset_returns, &benchmark_returns)
    }

    /// Tests for cointegration between two price series using close prices.
    pub fn cointegration(&self, a: &[Ohlcv], b: &[Ohlcv]) -> Result<CointegrationResult> {
        let (a_closes, b_closes) = align_two(a, b)?;
        cointegration(&a_closes, &b_closes)
    }

    /// Estimates the Hurst exponent from the close price series of `asset`.
    pub fn hurst(&self, asset: &[Ohlcv], max_lag: Option<usize>) -> Result<HurstResult> {
        let closes = extract_closes(asset)?;
        if closes.len() < 50 {
            return Err(QuantError::DataQuality(
                "Hurst estimation requires at least 50 aligned price observations".to_string(),
            ));
        }
        let prices: Vec<f64> = closes.into_iter().map(|(_, c)| c).collect();
        hurst_exponent(&prices, max_lag)
    }

    /// Performs PCA on the aligned return series of the supplied assets.
    pub fn pca(
        &self,
        assets: &HashMap<String, Vec<Ohlcv>>,
        n_components: usize,
    ) -> Result<PcaResult> {
        let (labels, aligned_closes) = align_many(assets)?;
        let returns_matrix: Vec<Vec<f64>> = aligned_closes
            .into_iter()
            .map(|closes| to_returns(&closes))
            .collect();
        let mut result = pca(&returns_matrix, n_components)?;
        result.labels = labels;
        Ok(result)
    }

    /// Computes a Pearson correlation matrix for the aligned return series.
    pub fn correlation(&self, assets: &HashMap<String, Vec<Ohlcv>>) -> Result<CorrelationMatrix> {
        let (labels, aligned_closes) = align_many(assets)?;
        let mut returns_map: HashMap<String, Vec<f64>> = HashMap::new();
        for (label, closes) in labels.into_iter().zip(aligned_closes) {
            returns_map.insert(label, to_returns(&closes));
        }
        correlation(&returns_map)
    }

    /// Runs a multi-factor regression of `asset` on the supplied factor series.
    pub fn multi_factor(
        &self,
        asset: &[Ohlcv],
        factors: &HashMap<String, Vec<Ohlcv>>,
    ) -> Result<MultiFactorResult> {
        let AlignedFactorReturns {
            asset_returns,
            factor_returns,
        } = align_asset_and_factors(asset, factors)?;
        multi_factor_regression(&asset_returns, &factor_returns)
    }

    /// Prices a European option with the Black-Scholes formula.
    pub fn black_scholes(&self, params: BlackScholesParams) -> Result<OptionPricingResult> {
        black_scholes(
            params.spot,
            params.strike,
            params.risk_free_rate,
            params.volatility,
            params.time_to_maturity,
            params.option_type,
        )
    }
}

fn extract_closes(ohlcv: &[Ohlcv]) -> Result<Vec<(NaiveDate, f64)>> {
    let mut out: Vec<(NaiveDate, f64)> = Vec::with_capacity(ohlcv.len());
    for bar in ohlcv {
        let close = bar.close.to_f64().ok_or_else(|| {
            QuantError::DataQuality(format!(
                "close price {} cannot be represented as f64",
                bar.close
            ))
        })?;
        out.push((bar.date, close));
    }
    out.sort_by_key(|(date, _)| *date);
    out.dedup_by_key(|(date, _)| *date);
    Ok(out)
}

fn to_returns(closes: &[f64]) -> Vec<f64> {
    closes.windows(2).map(|w| w[1] / w[0] - 1.0).collect()
}

fn align_two(a: &[Ohlcv], b: &[Ohlcv]) -> Result<(Vec<f64>, Vec<f64>)> {
    let a_map: HashMap<NaiveDate, f64> = extract_closes(a)?.into_iter().collect();
    let b_closes = extract_closes(b)?;

    let mut a_out = Vec::new();
    let mut b_out = Vec::new();
    for (date, b_close) in b_closes {
        if let Some(&a_close) = a_map.get(&date) {
            a_out.push(a_close);
            b_out.push(b_close);
        }
    }

    if a_out.len() < 2 {
        return Err(QuantError::DataQuality(
            "aligned series contains fewer than two common observations".to_string(),
        ));
    }

    Ok((a_out, b_out))
}

fn align_many(assets: &HashMap<String, Vec<Ohlcv>>) -> Result<(Vec<String>, Vec<Vec<f64>>)> {
    if assets.is_empty() {
        return Err(QuantError::DataQuality(
            "asset map must contain at least one series".to_string(),
        ));
    }

    // Intersect all date sets.
    let mut common_dates: Option<HashSet<NaiveDate>> = None;
    for (label, series) in assets {
        let dates: HashSet<NaiveDate> = extract_closes(series)?
            .into_iter()
            .map(|(d, _)| d)
            .collect();
        if dates.len() < 2 {
            return Err(QuantError::DataQuality(format!(
                "asset `{label}` has fewer than two observations"
            )));
        }
        common_dates = match common_dates {
            Some(set) => Some(set.intersection(&dates).copied().collect()),
            None => Some(dates),
        };
    }

    let mut dates: Vec<NaiveDate> = common_dates.unwrap().into_iter().collect();
    dates.sort();

    if dates.len() < 2 {
        return Err(QuantError::DataQuality(
            "assets share fewer than two common dates".to_string(),
        ));
    }

    let mut labels: Vec<String> = assets.keys().cloned().collect();
    labels.sort();

    let mut aligned: Vec<Vec<f64>> = Vec::with_capacity(labels.len());
    for label in &labels {
        let map: HashMap<NaiveDate, f64> = extract_closes(&assets[label])?.into_iter().collect();
        let closes: Vec<f64> = dates
            .iter()
            .map(|d| {
                map.get(d).copied().ok_or_else(|| {
                    QuantError::DataQuality(format!(
                        "date {d} missing for asset `{label}` after alignment"
                    ))
                })
            })
            .collect::<Result<_>>()?;
        aligned.push(closes);
    }

    Ok((labels, aligned))
}

/// Asset returns aligned with the return series of each factor.
#[derive(Debug, Clone)]
pub(crate) struct AlignedFactorReturns {
    /// Excess/simple returns of the asset.
    pub asset_returns: Vec<f64>,
    /// Return series for each factor, keyed by factor name.
    pub factor_returns: HashMap<String, Vec<f64>>,
}

fn align_asset_and_factors(
    asset: &[Ohlcv],
    factors: &HashMap<String, Vec<Ohlcv>>,
) -> Result<AlignedFactorReturns> {
    if factors.is_empty() {
        return Err(QuantError::DataQuality(
            "at least one factor is required".to_string(),
        ));
    }

    let asset_map: HashMap<NaiveDate, f64> = extract_closes(asset)?.into_iter().collect();
    let mut common_dates: HashSet<NaiveDate> = asset_map.keys().copied().collect();

    let mut factor_maps: HashMap<String, HashMap<NaiveDate, f64>> = HashMap::new();
    for (name, series) in factors {
        let map: HashMap<NaiveDate, f64> = extract_closes(series)?.into_iter().collect();
        if map.len() < 2 {
            return Err(QuantError::DataQuality(format!(
                "factor `{name}` has fewer than two observations"
            )));
        }
        common_dates = common_dates
            .intersection(&map.keys().copied().collect())
            .copied()
            .collect();
        factor_maps.insert(name.clone(), map);
    }

    let mut dates: Vec<NaiveDate> = common_dates.into_iter().collect();
    dates.sort();

    if dates.len() < 2 {
        return Err(QuantError::DataQuality(
            "asset and factors share fewer than two common dates".to_string(),
        ));
    }

    let asset_closes: Vec<f64> = dates
        .iter()
        .map(|d| {
            asset_map.get(d).copied().ok_or_else(|| {
                QuantError::DataQuality(format!("date {d} missing for asset after alignment"))
            })
        })
        .collect::<Result<_>>()?;
    let mut factor_returns: HashMap<String, Vec<f64>> = HashMap::new();
    for (name, map) in factor_maps {
        let closes: Vec<f64> = dates
            .iter()
            .map(|d| {
                map.get(d).copied().ok_or_else(|| {
                    QuantError::DataQuality(format!(
                        "date {d} missing for factor `{name}` after alignment"
                    ))
                })
            })
            .collect::<Result<_>>()?;
        factor_returns.insert(name, to_returns(&closes));
    }

    Ok(AlignedFactorReturns {
        asset_returns: to_returns(&asset_closes),
        factor_returns,
    })
}
