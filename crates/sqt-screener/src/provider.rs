//! Fundamental data providers.

use sqt_core::{QuantError, Result};

use crate::fundamental::FundamentalData;

/// A provider of fundamental data for a security universe.
pub trait FundamentalProvider: Send + Sync {
    /// Returns the complete fundamental dataset available from this provider.
    fn get_all(&self) -> Result<Vec<FundamentalData>>;
}

/// A hard-coded provider with a small static universe for tests and demos.
#[derive(Debug, Clone, Default)]
pub struct HardcodedFundamentalProvider;

impl HardcodedFundamentalProvider {
    /// Creates a new hard-coded provider.
    pub fn new() -> Self {
        Self
    }
}

impl FundamentalProvider for HardcodedFundamentalProvider {
    fn get_all(&self) -> Result<Vec<FundamentalData>> {
        Ok(vec![
            FundamentalData {
                ticker: "AAPL".to_string(),
                market_cap: 2_800e9,
                pe_ratio: 28.5,
                pb_ratio: 8.2,
                dividend_yield: 0.005,
                eps_growth: 0.12,
                debt_to_equity: 1.8,
                roe: 0.45,
            },
            FundamentalData {
                ticker: "MSFT".to_string(),
                market_cap: 2_500e9,
                pe_ratio: 32.0,
                pb_ratio: 12.0,
                dividend_yield: 0.007,
                eps_growth: 0.15,
                debt_to_equity: 0.5,
                roe: 0.40,
            },
            FundamentalData {
                ticker: "JPM".to_string(),
                market_cap: 450e9,
                pe_ratio: 11.0,
                pb_ratio: 1.4,
                dividend_yield: 0.025,
                eps_growth: 0.08,
                debt_to_equity: 1.5,
                roe: 0.14,
            },
            FundamentalData {
                ticker: "XOM".to_string(),
                market_cap: 450e9,
                pe_ratio: 13.0,
                pb_ratio: 1.9,
                dividend_yield: 0.032,
                eps_growth: 0.05,
                debt_to_equity: 0.3,
                roe: 0.16,
            },
            FundamentalData {
                ticker: "PFE".to_string(),
                market_cap: 170e9,
                pe_ratio: 9.5,
                pb_ratio: 1.6,
                dividend_yield: 0.040,
                eps_growth: 0.03,
                debt_to_equity: 0.7,
                roe: 0.18,
            },
            FundamentalData {
                ticker: "TSLA".to_string(),
                market_cap: 800e9,
                pe_ratio: 75.0,
                pb_ratio: 15.0,
                dividend_yield: 0.0,
                eps_growth: 0.25,
                debt_to_equity: 0.2,
                roe: 0.20,
            },
            FundamentalData {
                ticker: "V".to_string(),
                market_cap: 500e9,
                pe_ratio: 25.0,
                pb_ratio: 13.0,
                dividend_yield: 0.008,
                eps_growth: 0.14,
                debt_to_equity: 0.6,
                roe: 0.42,
            },
            FundamentalData {
                ticker: "KO".to_string(),
                market_cap: 260e9,
                pe_ratio: 23.0,
                pb_ratio: 10.0,
                dividend_yield: 0.028,
                eps_growth: 0.06,
                debt_to_equity: 1.6,
                roe: 0.32,
            },
            FundamentalData {
                ticker: "BAC".to_string(),
                market_cap: 280e9,
                pe_ratio: 10.5,
                pb_ratio: 1.0,
                dividend_yield: 0.026,
                eps_growth: 0.04,
                debt_to_equity: 1.4,
                roe: 0.10,
            },
            FundamentalData {
                ticker: "INTC".to_string(),
                market_cap: 140e9,
                pe_ratio: 16.0,
                pb_ratio: 1.3,
                dividend_yield: 0.018,
                eps_growth: -0.10,
                debt_to_equity: 0.4,
                roe: 0.08,
            },
        ])
    }
}

/// Convenience function to look up a single ticker from any provider.
///
/// This performs a linear scan over the provider's universe, so it is `O(n)`
/// in the number of securities.
pub fn get_by_ticker<P: FundamentalProvider>(
    provider: &P,
    ticker: &str,
) -> Result<FundamentalData> {
    provider
        .get_all()?
        .into_iter()
        .find(|d| d.ticker == ticker)
        .ok_or_else(|| QuantError::NotFound(format!("fundamental data for `{ticker}` not found")))
}
