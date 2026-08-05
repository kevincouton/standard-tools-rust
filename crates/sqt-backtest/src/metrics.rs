//! Shared equity-curve metrics for backtest engines.

use chrono::NaiveDate;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sqt_metrics::MetricsCalculator;

use crate::engine::BacktestConfig;

/// Computed metrics derived from an equity curve.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ComputedMetrics {
    /// Total return over the curve as a fraction of starting equity.
    pub total_return: f64,
    /// Maximum peak-to-trough drawdown as a negative fraction of equity.
    pub max_drawdown: f64,
    /// Annualised Sharpe ratio, if computable.
    pub sharpe: Option<f64>,
}

/// Computes total return, maximum drawdown and annualised Sharpe ratio from an
/// equity curve.
pub fn compute_metrics(
    equity_curve: &[(NaiveDate, Decimal)],
    config: BacktestConfig,
) -> ComputedMetrics {
    if equity_curve.is_empty() {
        return ComputedMetrics::default();
    }

    let initial = equity_curve.first().expect("equity curve is non-empty").1;
    let final_equity = equity_curve.last().expect("equity curve is non-empty").1;
    let total_return = if initial == Decimal::ZERO {
        0.0
    } else {
        let ratio = final_equity.to_f64().unwrap_or(0.0) / initial.to_f64().unwrap_or(1.0);
        ratio - 1.0
    };

    let returns: Vec<f64> = equity_curve
        .windows(2)
        .map(|window| {
            let prev = window[0].1;
            let curr = window[1].1;
            if prev == Decimal::ZERO {
                0.0
            } else {
                curr.to_f64().unwrap_or(0.0) / prev.to_f64().unwrap_or(1.0) - 1.0
            }
        })
        .collect();

    let max_drawdown = compute_max_drawdown(equity_curve);

    let sharpe = MetricsCalculator::from_returns(
        &returns,
        config.risk_free_rate,
        None,
        config.periods_per_year,
    )
    .ok()
    .and_then(|m| m.sharpe);

    ComputedMetrics {
        total_return,
        max_drawdown,
        sharpe,
    }
}

/// Computes the maximum peak-to-trough drawdown as a negative fraction.
pub fn compute_max_drawdown(equity_curve: &[(NaiveDate, Decimal)]) -> f64 {
    let mut peak = Decimal::ZERO;
    let mut max_dd = Decimal::ZERO;
    for (_, equity) in equity_curve {
        if *equity > peak {
            peak = *equity;
        }
        if peak > Decimal::ZERO {
            let drawdown = (peak - *equity) / peak;
            if drawdown > max_dd {
                max_dd = drawdown;
            }
        }
    }
    -max_dd.to_f64().unwrap_or(0.0)
}
