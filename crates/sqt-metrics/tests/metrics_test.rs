//! Unit tests for the `sqt-metrics` crate.

use sqt_metrics::MetricsCalculator;

const EPSILON: f64 = 1e-6;

#[test]
fn total_return_and_volatility_with_alternating_returns() {
    let returns = vec![0.01, -0.01, 0.01, -0.01];
    let result = MetricsCalculator::from_returns(&returns, 0.02, None, 252).unwrap();

    assert!((result.total_return.unwrap() - -0.00019999).abs() < EPSILON);
    assert!(
        (result.annualized_return.unwrap() - -0.0125215745).abs() < EPSILON,
        "annualized_return = {:?}",
        result.annualized_return
    );
    assert!(
        (result.volatility.unwrap() - 0.1587450787).abs() < EPSILON,
        "volatility = {:?}",
        result.volatility
    );
    assert!(
        (result.sharpe.unwrap() - -0.2048666628).abs() < EPSILON,
        "sharpe = {:?}",
        result.sharpe
    );
    assert!(
        (result.sortino.unwrap() - -0.2874439121).abs() < EPSILON,
        "sortino = {:?}",
        result.sortino
    );
    assert!(
        (result.max_drawdown.unwrap() - -0.01).abs() < EPSILON,
        "max_drawdown = {:?}",
        result.max_drawdown
    );
    assert!(
        (result.var.unwrap() - -0.01).abs() < EPSILON,
        "var = {:?}",
        result.var
    );
    assert!(
        (result.cvar.unwrap() - -0.01).abs() < EPSILON,
        "cvar = {:?}",
        result.cvar
    );
    assert!(result.beta.is_none());
    assert!(result.alpha.is_none());
}

#[test]
fn constant_positive_return_has_zero_volatility_and_no_sharpe() {
    let returns = vec![0.001; 252];
    let result = MetricsCalculator::from_returns(&returns, 0.02, None, 252).unwrap();

    assert!(
        (result.total_return.unwrap() - 0.2864340444).abs() < EPSILON,
        "total_return = {:?}",
        result.total_return
    );
    assert!(
        (result.annualized_return.unwrap() - 0.2864340444).abs() < EPSILON,
        "annualized_return = {:?}",
        result.annualized_return
    );
    assert!(
        result.volatility.unwrap().abs() < EPSILON,
        "volatility = {:?}",
        result.volatility
    );
    assert!(result.sharpe.is_none(), "sharpe should be None");
    assert!(result.sortino.is_none(), "sortino should be None");
    assert!(
        result.max_drawdown.unwrap().abs() < EPSILON,
        "max_drawdown = {:?}",
        result.max_drawdown
    );
}

#[test]
fn negative_returns_produce_negative_metrics() {
    let returns = vec![-0.05, -0.05, -0.05];
    let result = MetricsCalculator::from_returns(&returns, 0.02, None, 252).unwrap();

    assert!(
        (result.total_return.unwrap() - -0.142625).abs() < EPSILON,
        "total_return = {:?}",
        result.total_return
    );
    assert!(
        (result.max_drawdown.unwrap() - -0.05).abs() < EPSILON,
        "max_drawdown = {:?}",
        result.max_drawdown
    );
    assert!(
        (result.var.unwrap() - -0.05).abs() < EPSILON,
        "var = {:?}",
        result.var
    );
    assert!(
        (result.cvar.unwrap() - -0.05).abs() < EPSILON,
        "cvar = {:?}",
        result.cvar
    );
}

#[test]
fn benchmark_computes_beta_and_alpha() {
    let returns = vec![0.02, 0.01, -0.01, 0.015, 0.005];
    let benchmark = vec![0.01, 0.01, -0.005, 0.01, 0.0];
    let result = MetricsCalculator::from_returns(&returns, 0.02, Some(&benchmark), 252).unwrap();

    assert!(
        (result.beta.unwrap() - 1.5).abs() < EPSILON,
        "beta = {:?}",
        result.beta
    );
    assert!(
        (result.alpha.unwrap() - 2.6152639789).abs() < 1e-4,
        "alpha = {:?}",
        result.alpha
    );
    assert!(
        result.total_return.unwrap() > 0.0,
        "total_return = {:?}",
        result.total_return
    );
}

#[test]
fn empty_returns_yield_all_none() {
    let result = MetricsCalculator::from_returns(&[], 0.02, None, 252).unwrap();
    assert!(result.total_return.is_none());
    assert!(result.annualized_return.is_none());
    assert!(result.volatility.is_none());
    assert!(result.sharpe.is_none());
    assert!(result.sortino.is_none());
    assert!(result.max_drawdown.is_none());
    assert!(result.var.is_none());
    assert!(result.cvar.is_none());
    assert!(result.beta.is_none());
    assert!(result.alpha.is_none());
}

#[test]
fn mismatched_benchmark_length_returns_error() {
    let returns = vec![0.01, 0.02];
    let benchmark = vec![0.01];
    let err = MetricsCalculator::from_returns(&returns, 0.02, Some(&benchmark), 252).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::InvalidCommand(_)));
}

#[test]
fn monthly_periods_scale_metrics_by_periods_per_year() {
    // 12 monthly returns of exactly 1% => total return ~12.68%, annualised ~12.68%.
    let returns = vec![0.01; 12];
    let result = MetricsCalculator::from_returns(&returns, 0.02, None, 12).unwrap();

    assert!(
        (result.total_return.unwrap() - 0.12682503).abs() < 1e-6,
        "total_return = {:?}",
        result.total_return
    );
    assert!(
        (result.annualized_return.unwrap() - 0.12682503).abs() < 1e-6,
        "annualized_return = {:?}",
        result.annualized_return
    );
    assert!(
        result.volatility.unwrap().abs() < EPSILON,
        "volatility should be zero for constant returns, got {:?}",
        result.volatility
    );
}

#[test]
fn realistic_market_style_metrics() {
    // 21 trading days (~one month) of modest daily returns with a small drawdown.
    let returns = vec![
        0.005, 0.002, -0.004, 0.003, 0.001, -0.006, 0.004, 0.002, 0.0, 0.003, -0.001, 0.002,
        -0.003, 0.005, 0.001, -0.002, 0.004, 0.003, -0.001, 0.002, 0.001,
    ];
    let result = MetricsCalculator::from_returns(&returns, 0.02, None, 252).unwrap();

    assert!(result.total_return.unwrap() > 0.0, "positive total return");
    assert!(result.volatility.unwrap() > 0.0, "positive volatility");
    assert!(result.max_drawdown.unwrap() < 0.0, "negative max drawdown");
    assert!(result.var.unwrap() < 0.0, "negative VaR");
    assert!(result.cvar.unwrap() <= result.var.unwrap(), "CVaR <= VaR");
}
