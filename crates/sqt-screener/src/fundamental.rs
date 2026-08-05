//! Fundamental data types and screening filters.

use serde::{Deserialize, Serialize};

/// A snapshot of fundamental metrics for a single security.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FundamentalData {
    /// Ticker symbol, e.g. `"AAPL"`.
    pub ticker: String,
    /// Market capitalisation in the reporting currency.
    pub market_cap: f64,
    /// Price-to-earnings ratio.
    pub pe_ratio: f64,
    /// Price-to-book ratio.
    pub pb_ratio: f64,
    /// Dividend yield as a decimal (e.g. `0.03` for 3%).
    pub dividend_yield: f64,
    /// Earnings-per-share growth as a decimal.
    pub eps_growth: f64,
    /// Debt-to-equity ratio.
    pub debt_to_equity: f64,
    /// Return on equity as a decimal.
    pub roe: f64,
}

/// A single fundamental screening condition.
#[derive(Debug, Clone, PartialEq)]
pub enum FundamentalFilter {
    /// Price-to-earnings ratio less than the given value.
    PeLt(f64),
    /// Price-to-earnings ratio greater than the given value.
    PeGt(f64),
    /// Market capitalisation greater than the given value.
    MarketCapGt(f64),
    /// Market capitalisation less than the given value.
    MarketCapLt(f64),
    /// Price-to-book ratio less than the given value.
    PbLt(f64),
    /// Price-to-book ratio greater than the given value.
    PbGt(f64),
    /// Dividend yield greater than the given value.
    DividendYieldGt(f64),
    /// Dividend yield less than the given value.
    DividendYieldLt(f64),
    /// EPS growth greater than the given value.
    EpsGrowthGt(f64),
    /// EPS growth less than the given value.
    EpsGrowthLt(f64),
    /// Debt-to-equity ratio less than the given value.
    DebtToEquityLt(f64),
    /// Debt-to-equity ratio greater than the given value.
    DebtToEquityGt(f64),
    /// Return on equity greater than the given value.
    RoeGt(f64),
    /// Return on equity less than the given value.
    RoeLt(f64),
}

impl FundamentalFilter {
    /// Returns `true` if `data` satisfies this filter condition.
    ///
    /// If the metric required by the filter is `NaN`, the security is excluded
    /// and this method returns `false`.
    pub fn apply(&self, data: &FundamentalData) -> bool {
        match self {
            FundamentalFilter::PeLt(v) => data.pe_ratio.is_finite() && data.pe_ratio < *v,
            FundamentalFilter::PeGt(v) => data.pe_ratio.is_finite() && data.pe_ratio > *v,
            FundamentalFilter::MarketCapGt(v) => {
                data.market_cap.is_finite() && data.market_cap > *v
            }
            FundamentalFilter::MarketCapLt(v) => {
                data.market_cap.is_finite() && data.market_cap < *v
            }
            FundamentalFilter::PbLt(v) => data.pb_ratio.is_finite() && data.pb_ratio < *v,
            FundamentalFilter::PbGt(v) => data.pb_ratio.is_finite() && data.pb_ratio > *v,
            FundamentalFilter::DividendYieldGt(v) => {
                data.dividend_yield.is_finite() && data.dividend_yield > *v
            }
            FundamentalFilter::DividendYieldLt(v) => {
                data.dividend_yield.is_finite() && data.dividend_yield < *v
            }
            FundamentalFilter::EpsGrowthGt(v) => {
                data.eps_growth.is_finite() && data.eps_growth > *v
            }
            FundamentalFilter::EpsGrowthLt(v) => {
                data.eps_growth.is_finite() && data.eps_growth < *v
            }
            FundamentalFilter::DebtToEquityLt(v) => {
                data.debt_to_equity.is_finite() && data.debt_to_equity < *v
            }
            FundamentalFilter::DebtToEquityGt(v) => {
                data.debt_to_equity.is_finite() && data.debt_to_equity > *v
            }
            FundamentalFilter::RoeGt(v) => data.roe.is_finite() && data.roe > *v,
            FundamentalFilter::RoeLt(v) => data.roe.is_finite() && data.roe < *v,
        }
    }
}
