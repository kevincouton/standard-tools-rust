//! Integration tests for the `sqt-portfolio` optimizers and service.

use std::collections::HashMap;

use sqt_portfolio::{
    black_litterman_optimize, black_litterman_optimize_simplified, mean_variance_optimize,
    risk_parity_optimize, PortfolioService,
};

fn labels() -> Vec<String> {
    vec!["AAPL".to_string(), "MSFT".to_string(), "TSLA".to_string()]
}

fn returns_matrix() -> Vec<Vec<f64>> {
    // Three assets, 12 monthly observations.
    vec![
        // AAPL: high return, low volatility.
        vec![
            0.010, 0.012, 0.009, 0.011, 0.010, 0.013, 0.009, 0.012, 0.010, 0.011, 0.010, 0.012,
        ],
        // MSFT: medium return, low volatility.
        vec![
            0.005, 0.006, 0.004, 0.005, 0.006, 0.005, 0.004, 0.006, 0.005, 0.005, 0.006, 0.004,
        ],
        // TSLA: medium return, high volatility.
        vec![
            -0.005, 0.015, -0.010, 0.020, -0.008, 0.018, -0.012, 0.022, -0.007, 0.016, -0.009,
            0.019,
        ],
    ]
}

fn market_caps() -> HashMap<String, f64> {
    [
        ("AAPL".to_string(), 1_000.0),
        ("MSFT".to_string(), 500.0),
        ("TSLA".to_string(), 200.0),
    ]
    .into_iter()
    .collect()
}

#[test]
fn mean_variance_max_sharpe_chooses_highest_return_asset() {
    let result = mean_variance_optimize(&returns_matrix(), &labels(), 0.0, None).unwrap();

    let aapl = result.weights["AAPL"];
    let total: f64 = result.weights.values().sum();

    assert!(
        aapl > result.weights["MSFT"] && aapl > result.weights["TSLA"],
        "AAPL should receive the largest allocation in the max-Sharpe portfolio"
    );
    assert!(
        (total - 1.0).abs() < 1e-9,
        "weights should sum to one, got {total}"
    );
    assert!(result.sharpe.is_finite());
    assert!(result.volatility >= 0.0);
}

#[test]
fn mean_variance_target_return_blends_portfolios() {
    let max_sharpe = mean_variance_optimize(&returns_matrix(), &labels(), 0.0, None).unwrap();
    let min_return = 0.006;
    let target_return = Some(min_return);
    let result = mean_variance_optimize(&returns_matrix(), &labels(), 0.0, target_return).unwrap();

    // With a target below the max-Sharpe return, the optimizer should blend
    // toward the minimum-variance portfolio, producing a lower volatility than
    // the pure max-Sharpe portfolio.
    assert!(
        result.volatility < max_sharpe.volatility * 1.5,
        "target-return portfolio volatility unexpectedly high"
    );
    assert!(
        (result.expected_return - min_return).abs() < 0.01 || result.expected_return >= min_return,
        "expected return should be at least the target when feasible"
    );
}

#[test]
fn risk_parity_weights_sum_to_one() {
    let result = risk_parity_optimize(&returns_matrix(), &labels()).unwrap();
    let total: f64 = result.weights.values().sum();
    assert!(
        (total - 1.0).abs() < 1e-9,
        "risk-parity weights should sum to one"
    );

    // Low-volatility AAPL should receive a larger weight than high-volatility TSLA.
    assert!(
        result.weights["AAPL"] > result.weights["TSLA"],
        "low-volatility asset should be overweighted"
    );
}

#[test]
fn black_litterman_explicit_views_produce_valid_weights() {
    let p = vec![
        vec![1.0, 0.0, 0.0], // AAPL expected return view
    ];
    let q = vec![0.02];
    let caps = vec![1_000.0, 500.0, 200.0];

    let result =
        black_litterman_optimize(&returns_matrix(), &labels(), &caps, &p, &q, 0.05, 2.5).unwrap();

    let total: f64 = result.weights.values().sum();
    assert!(
        (total - 1.0).abs() < 1e-6,
        "BL weights should sum to one, got {total}"
    );

    assert_eq!(result.weights.len(), 3);
    assert_eq!(result.expected_returns.len(), 3);
    assert_eq!(result.covariance.len(), 3);
    assert!(result.covariance.iter().all(|row| row.len() == 3));
}

#[test]
fn black_litterman_simplified_maps_views_by_ticker() {
    let mut views = HashMap::new();
    views.insert("AAPL".to_string(), 0.02);

    let result = black_litterman_optimize_simplified(
        &labels(),
        &returns_matrix(),
        &market_caps(),
        &views,
        0.05,
        2.5,
    )
    .unwrap();

    let total: f64 = result.weights.values().sum();
    assert!((total - 1.0).abs() < 1e-6);
    assert!(result.weights.contains_key("AAPL"));
}

#[test]
fn portfolio_service_routes_to_optimizers() {
    let mut returns: HashMap<String, Vec<f64>> = HashMap::new();
    for (label, series) in labels().into_iter().zip(returns_matrix()) {
        returns.insert(label, series);
    }

    let service = PortfolioService::new();

    let mv = service.mean_variance(&returns, 0.0, None).unwrap();
    assert!(mv.weights.contains_key("AAPL"));

    let rp = service.risk_parity(&returns).unwrap();
    assert!((rp.weights.values().sum::<f64>() - 1.0).abs() < 1e-9);

    let mut views = HashMap::new();
    views.insert("AAPL".to_string(), 0.02);
    let bl = service
        .black_litterman(&returns, &market_caps(), &views, 0.05, 2.5)
        .unwrap();
    assert!((bl.weights.values().sum::<f64>() - 1.0).abs() < 1e-6);
}

#[test]
fn mean_variance_two_asset_hand_verified() {
    // Two uncorrelated assets with analytically tractable max-Sharpe weights.
    // Asset A: mean 5%, variance 0.0001
    // Asset B: mean 10%, variance 0.0003
    // With rf = 0, max-Sharpe weights are proportional to mean/variance:
    //   w_A ∝ 0.05 / 0.0001 = 500
    //   w_B ∝ 0.10 / 0.0003 = 333.33
    // Normalised: w_A = 0.6, w_B = 0.4.
    let labels = vec!["A".to_string(), "B".to_string()];
    let returns = vec![vec![0.06, 0.04, 0.05], vec![0.11, 0.11, 0.08]];

    let result = mean_variance_optimize(&returns, &labels, 0.0, None).unwrap();

    let total: f64 = result.weights.values().sum();
    assert!((total - 1.0).abs() < 1e-9);
    assert!((result.weights["A"] - 0.6).abs() < 1e-4);
    assert!((result.weights["B"] - 0.4).abs() < 1e-4);
    assert!((result.expected_return - 0.07).abs() < 1e-4);
    assert!((result.volatility - 0.00916515).abs() < 1e-6);
    assert!((result.sharpe - 7.6376).abs() < 1e-3);
}

#[test]
fn black_litterman_positive_view_raises_weight_above_market_cap() {
    let labels = labels();
    let returns = returns_matrix();
    let caps = vec![1_000.0, 500.0, 200.0];

    // Strong positive view on AAPL: expected return far above equilibrium.
    let p = vec![vec![1.0, 0.0, 0.0]];
    let q = vec![0.10];

    let result = black_litterman_optimize(&returns, &labels, &caps, &p, &q, 0.05, 2.5).unwrap();

    let aapl_market_cap_weight = 1_000.0 / (1_000.0 + 500.0 + 200.0);
    assert!(
        result.weights["AAPL"] > aapl_market_cap_weight,
        "AAPL BL weight should exceed market-cap weight when view is strongly positive"
    );
}

#[test]
fn mean_variance_rejects_empty_inputs() {
    let err = mean_variance_optimize(&[], &[], 0.0, None).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn mean_variance_rejects_mismatched_dimensions() {
    let labels = vec!["A".to_string(), "B".to_string()];
    let returns = vec![vec![0.01, 0.02]];
    let err = mean_variance_optimize(&returns, &labels, 0.0, None).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::InvalidCommand(_)));
}

#[test]
fn mean_variance_rejects_non_finite_returns() {
    let labels = vec!["A".to_string(), "B".to_string()];
    let returns = vec![vec![0.01, f64::NAN], vec![0.02, 0.03]];
    let err = mean_variance_optimize(&returns, &labels, 0.0, None).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn mean_variance_rejects_non_finite_target_return() {
    let labels = vec!["A".to_string(), "B".to_string()];
    let returns = vec![vec![0.01, 0.02], vec![0.02, 0.03]];
    let err = mean_variance_optimize(&returns, &labels, 0.0, Some(f64::INFINITY)).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::InvalidCommand(_)));
}

#[test]
fn mean_variance_rejects_duplicate_labels() {
    let labels = vec!["A".to_string(), "A".to_string()];
    let returns = vec![vec![0.01, 0.02], vec![0.02, 0.03]];
    let err = mean_variance_optimize(&returns, &labels, 0.0, None).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::InvalidCommand(_)));
}

#[test]
fn risk_parity_rejects_all_zero_volatilities() {
    // Constant return series -> zero sample volatility for every asset.
    let labels = vec!["A".to_string(), "B".to_string()];
    let returns = vec![vec![0.01, 0.01], vec![0.02, 0.02]];
    let err = risk_parity_optimize(&returns, &labels).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn black_litterman_rejects_missing_market_caps() {
    let labels = vec!["AAPL".to_string(), "MSFT".to_string()];
    let returns = vec![vec![0.01, 0.02], vec![0.02, 0.03]];
    let mut caps = HashMap::new();
    caps.insert("AAPL".to_string(), 1_000.0);
    // MSFT missing.
    let mut views = HashMap::new();
    views.insert("AAPL".to_string(), 0.05);
    let err = black_litterman_optimize_simplified(&labels, &returns, &caps, &views, 0.05, 2.5)
        .unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::InvalidCommand(_)));
}

#[test]
fn black_litterman_rejects_unknown_view_ticker() {
    let labels = vec!["AAPL".to_string(), "MSFT".to_string()];
    let returns = vec![vec![0.01, 0.02], vec![0.02, 0.03]];
    let mut caps = HashMap::new();
    caps.insert("AAPL".to_string(), 1_000.0);
    caps.insert("MSFT".to_string(), 500.0);
    let mut views = HashMap::new();
    views.insert("UNKNOWN".to_string(), 0.05);
    let err = black_litterman_optimize_simplified(&labels, &returns, &caps, &views, 0.05, 2.5)
        .unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::InvalidCommand(_)));
}

#[test]
fn black_litterman_rejects_duplicate_labels() {
    let labels = vec!["AAPL".to_string(), "AAPL".to_string()];
    let returns = vec![vec![0.01, 0.02], vec![0.02, 0.03]];
    let caps = vec![1_000.0, 500.0];
    let p = vec![vec![1.0, 0.0]];
    let q = vec![0.05];
    let err = black_litterman_optimize(&returns, &labels, &caps, &p, &q, 0.05, 2.5).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::InvalidCommand(_)));
}
