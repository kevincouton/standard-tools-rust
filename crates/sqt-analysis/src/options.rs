//! Option pricing and Greeks via the Black-Scholes model.
//!
//! All Greeks are expressed in the natural units of the inputs: delta is a
//! percentage of the underlying, gamma is per unit of spot, vega is per unit
//! (100 percentage points) of volatility, theta is per year, and rho is per unit
//! (100 percentage points) of the risk-free rate.

use sqt_core::{QuantError, Result};
use statrs::distribution::{Continuous, ContinuousCDF, Normal};

use crate::math::validate_finite;

/// Type of option contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionType {
    /// Call option.
    Call,
    /// Put option.
    Put,
}

/// Parameters required to price an option with the Black-Scholes formula.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlackScholesParams {
    /// Current spot price of the underlying.
    pub spot: f64,
    /// Strike price of the option.
    pub strike: f64,
    /// Risk-free interest rate (continuously compounded, annualised).
    pub risk_free_rate: f64,
    /// Annualised volatility of the underlying.
    pub volatility: f64,
    /// Time to maturity in years.
    pub time_to_maturity: f64,
    /// Option type (call or put).
    pub option_type: OptionType,
}

/// Result of a Black-Scholes valuation.
#[derive(Debug, Clone, PartialEq)]
pub struct OptionPricingResult {
    /// Theoretical option price.
    pub price: f64,
    /// Delta sensitivity.
    pub delta: f64,
    /// Gamma sensitivity.
    pub gamma: f64,
    /// Vega sensitivity.
    pub vega: f64,
    /// Theta sensitivity.
    pub theta: f64,
    /// Rho sensitivity.
    pub rho: f64,
}

/// Prices a European option using the Black-Scholes formula.
///
/// All inputs must be non-negative finite numbers and `time_to_maturity` must be
/// positive. `volatility` is expected as an annualised decimal (e.g. `0.20` for
/// 20%).
///
/// # Errors
///
/// Returns [`QuantError::DataQuality`] if any input is invalid.
pub fn black_scholes(
    spot: f64,
    strike: f64,
    risk_free_rate: f64,
    volatility: f64,
    time_to_maturity: f64,
    option_type: OptionType,
) -> Result<OptionPricingResult> {
    validate_finite(&[spot, strike, risk_free_rate, volatility, time_to_maturity])?;
    for (name, value) in [
        ("spot", spot),
        ("strike", strike),
        ("risk_free_rate", risk_free_rate),
        ("volatility", volatility),
        ("time_to_maturity", time_to_maturity),
    ] {
        if value < 0.0 {
            return Err(QuantError::DataQuality(format!(
                "{name} must be a non-negative finite number, got {value}"
            )));
        }
    }
    if time_to_maturity == 0.0 {
        return Err(QuantError::DataQuality(
            "time_to_maturity must be positive".to_string(),
        ));
    }
    if volatility == 0.0 {
        return Err(QuantError::DataQuality(
            "volatility must be positive".to_string(),
        ));
    }

    let normal = Normal::new(0.0, 1.0).map_err(|e| {
        QuantError::Internal(anyhow::anyhow!("failed to create normal distribution: {e}"))
    })?;

    let sqrt_t = time_to_maturity.sqrt();
    let d1 = ((spot / strike).ln()
        + (risk_free_rate + 0.5 * volatility * volatility) * time_to_maturity)
        / (volatility * sqrt_t);
    let d2 = d1 - volatility * sqrt_t;

    let nd1 = normal.cdf(d1);
    let nd2 = normal.cdf(d2);
    let n_d1 = normal.cdf(-d1);
    let n_d2 = normal.cdf(-d2);
    let pdf_d1 = normal.pdf(d1);

    let discount = (-risk_free_rate * time_to_maturity).exp();

    let (price, delta, theta, rho) = match option_type {
        OptionType::Call => {
            let price = spot * nd1 - strike * discount * nd2;
            let delta = nd1;
            let theta = -spot * pdf_d1 * volatility / (2.0 * sqrt_t)
                - risk_free_rate * strike * discount * nd2;
            let rho = strike * time_to_maturity * discount * nd2;
            (price, delta, theta, rho)
        }
        OptionType::Put => {
            let price = strike * discount * n_d2 - spot * n_d1;
            let delta = nd1 - 1.0;
            let theta = -spot * pdf_d1 * volatility / (2.0 * sqrt_t)
                + risk_free_rate * strike * discount * n_d2;
            let rho = -strike * time_to_maturity * discount * n_d2;
            (price, delta, theta, rho)
        }
    };

    let gamma = pdf_d1 / (spot * volatility * sqrt_t);
    let vega = spot * pdf_d1 * sqrt_t;

    Ok(OptionPricingResult {
        price,
        delta,
        gamma,
        vega,
        theta,
        rho,
    })
}
