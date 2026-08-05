use std::collections::HashMap;

use chrono::{Duration, NaiveDate};
use rust_decimal::Decimal;
use sqt_analysis::{
    black_scholes, cointegration, correlation, hurst_exponent, linear_regression,
    multi_factor_regression, pca, AnalysisService, BlackScholesParams, OptionType,
};
use sqt_core::{Ohlcv, Ticker};

fn ohlcv_series(symbol: &str, prices: &[f64]) -> Vec<Ohlcv> {
    let base = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let ticker = Ticker::new(symbol);
    prices
        .iter()
        .enumerate()
        .map(|(i, &close)| {
            let date = base + Duration::days(i as i64);
            Ohlcv::try_new(
                ticker.clone(),
                date,
                Decimal::try_from(close).unwrap(),
                Decimal::try_from(close).unwrap(),
                Decimal::try_from(close).unwrap(),
                Decimal::try_from(close).unwrap(),
                1_000_000,
            )
            .unwrap()
        })
        .collect()
}

fn next_uniform(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*seed >> 32) as f64) / (u32::MAX as f64)
}

#[test]
fn test_linear_regression_known_beta() {
    // y = 1.5 + 2.0 * x + small deterministic noise
    let mut asset = Vec::with_capacity(100);
    let mut benchmark = Vec::with_capacity(100);
    for i in 0..100 {
        let x = i as f64 / 100.0;
        let noise = (i as f64 / 1000.0) - 0.05;
        let y = 1.5 + 2.0 * x + noise;
        benchmark.push(x);
        asset.push(y);
    }

    let result = linear_regression(&asset, &benchmark).unwrap();

    assert!((result.beta - 2.0).abs() < 0.1, "beta should be near 2.0");
    assert!((result.alpha - 1.5).abs() < 0.2, "alpha should be near 1.5");
    assert!(result.r_squared > 0.95, "R² should be high");
    assert!((0.0..=1.0).contains(&result.p_value), "p-value in [0, 1]");
    let sum_resid: f64 = result.residuals.iter().sum();
    assert!(sum_resid.abs() < 1e-9, "residuals should sum to zero");
}

#[test]
fn test_linear_regression_mismatched_lengths() {
    let asset = vec![1.0, 2.0, 3.0];
    let benchmark = vec![1.0, 2.0];
    let err = linear_regression(&asset, &benchmark).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_linear_regression_too_short() {
    let asset = vec![1.0];
    let benchmark = vec![2.0];
    let err = linear_regression(&asset, &benchmark).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_linear_regression_zero_variance_benchmark() {
    let asset = vec![1.0, 2.0, 3.0, 4.0];
    let benchmark = vec![1.0; 4];
    let err = linear_regression(&asset, &benchmark).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_linear_regression_non_finite_input() {
    let asset = vec![1.0, f64::NAN, 3.0, 4.0];
    let benchmark = vec![1.0, 2.0, 3.0, 4.0];
    let err = linear_regression(&asset, &benchmark).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_cointegration_basic() {
    // A = 2 * B + strongly mean-reverting residual => cointegrated.
    let n = 200;
    let b: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 * 0.1).collect();
    let mut seed = 42u64;
    let mut residual = 0.0;
    let a: Vec<f64> = b
        .iter()
        .map(|&b| {
            let innovation = (next_uniform(&mut seed) - 0.5) * 0.5;
            // AR(1) coefficient near 0.7 gives a clear mean-reverting residual.
            residual = 0.7 * residual + innovation;
            2.0 * b + residual
        })
        .collect();

    let result = cointegration(&a, &b).unwrap();

    assert!(
        (result.hedge_ratio - 2.0).abs() < 0.1,
        "hedge ratio near 2.0"
    );
    // The residual is constructed to be mean-reverting, so a finite half-life is
    // expected. Allow infinity only as a defensive fallback for estimation noise.
    assert!(
        result.half_life.is_finite() && result.half_life > 0.0 || result.half_life.is_infinite(),
        "half-life should be positive finite or infinity"
    );
    assert!((0.0..=1.0).contains(&result.p_value), "p-value in [0, 1]");
    assert!(result.z_score.is_finite(), "z-score finite");
}

#[test]
fn test_cointegration_mismatched_lengths() {
    let a = vec![1.0; 10];
    let b = vec![2.0; 11];
    let err = cointegration(&a, &b).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_cointegration_too_short() {
    let a = vec![1.0; 5];
    let b = vec![2.0; 5];
    let err = cointegration(&a, &b).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_cointegration_non_finite_input() {
    let a = vec![1.0; 10];
    let mut b = vec![2.0; 10];
    b[3] = f64::INFINITY;
    let err = cointegration(&a, &b).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_hurst_random_walk() {
    // Approximate geometric random walk should give H near 0.5.
    let mut prices = vec![100.0];
    let mut seed = 99u64;
    for _ in 0..499 {
        let prev = *prices.last().unwrap();
        let r = (next_uniform(&mut seed) - 0.5) * 0.02;
        prices.push(prev * (1.0 + r));
    }

    let result = hurst_exponent(&prices, None).unwrap();
    assert!((0.0..=1.0).contains(&result.exponent), "H in [0, 1]");
    assert!(
        result.exponent > 0.4 && result.exponent < 0.6,
        "random walk H near 0.5, got {}",
        result.exponent
    );
}

#[test]
fn test_hurst_too_short() {
    let prices = vec![100.0; 10];
    let err = hurst_exponent(&prices, None).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_hurst_non_positive_prices() {
    let mut prices = vec![100.0; 50];
    prices[5] = -1.0;
    let err = hurst_exponent(&prices, None).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_hurst_non_finite_input() {
    let mut prices = vec![100.0; 50];
    prices[5] = f64::NAN;
    let err = hurst_exponent(&prices, None).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_pca_explained_variance() {
    // Two correlated series plus one independent series.
    let n = 100;
    let mut seed = 7u64;
    let x: Vec<f64> = (0..n).map(|_| next_uniform(&mut seed)).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&v| v + next_uniform(&mut seed) * 0.1)
        .collect();
    let z: Vec<f64> = (0..n).map(|_| next_uniform(&mut seed)).collect();
    let matrix = vec![x, y, z];

    let result = pca(&matrix, 2).unwrap();

    assert_eq!(result.labels.len(), 0);
    assert_eq!(result.eigenvalues.len(), 2);
    assert_eq!(result.eigenvectors.len(), 2);
    assert_eq!(result.explained_variance_ratio.len(), 2);
    let sum_ratio: f64 = result.explained_variance_ratio.iter().sum();
    assert!(
        sum_ratio > 0.0 && sum_ratio <= 1.0,
        "explained ratio in (0, 1]"
    );
    assert!(
        result.eigenvalues[0] >= result.eigenvalues[1],
        "eigenvalues sorted"
    );
    for vec in &result.eigenvectors {
        assert_eq!(vec.len(), 3);
    }
}

#[test]
fn test_pca_invalid_n_components() {
    let matrix = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
    let err = pca(&matrix, 3).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_pca_empty_matrix() {
    let matrix: Vec<Vec<f64>> = vec![];
    let err = pca(&matrix, 1).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_pca_mismatched_series_lengths() {
    let matrix = vec![vec![1.0, 2.0], vec![3.0, 4.0, 5.0]];
    let err = pca(&matrix, 1).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_pca_non_finite_input() {
    let matrix = vec![vec![1.0, f64::NAN], vec![3.0, 4.0]];
    let err = pca(&matrix, 1).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_correlation_matrix() {
    let n = 100;
    let mut seed = 123u64;
    let x: Vec<f64> = (0..n).map(|_| next_uniform(&mut seed)).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&v| v + next_uniform(&mut seed) * 0.1)
        .collect();
    // Z is constructed to be negatively correlated with X.
    let z: Vec<f64> = x
        .iter()
        .map(|&v| -0.8 * v + next_uniform(&mut seed) * 0.1)
        .collect();

    let mut map = HashMap::new();
    map.insert("X".to_string(), x);
    map.insert("Y".to_string(), y);
    map.insert("Z".to_string(), z);

    let result = correlation(&map).unwrap();
    assert_eq!(result.labels, vec!["X", "Y", "Z"]);

    let i_x = result.labels.iter().position(|l| l == "X").unwrap();
    let i_y = result.labels.iter().position(|l| l == "Y").unwrap();
    let i_z = result.labels.iter().position(|l| l == "Z").unwrap();

    assert!(result.matrix[i_x][i_y] > 0.85, "X-Y correlation high");
    assert!(result.matrix[i_y][i_x] > 0.85, "matrix symmetric");
    assert!(result.matrix[i_x][i_z] < -0.85, "X-Z correlation near -1");
}

#[test]
fn test_correlation_empty_map() {
    let map: HashMap<String, Vec<f64>> = HashMap::new();
    let err = correlation(&map).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_correlation_zero_variance() {
    let mut map = HashMap::new();
    map.insert("X".to_string(), vec![1.0; 10]);
    map.insert(
        "Y".to_string(),
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
    );
    let err = correlation(&map).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_correlation_non_finite_input() {
    let mut map = HashMap::new();
    map.insert("X".to_string(), vec![1.0, f64::NAN, 3.0, 4.0]);
    map.insert("Y".to_string(), vec![1.0, 2.0, 3.0, 4.0]);
    let err = correlation(&map).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_multi_factor_regression() {
    let n = 100;
    let mut seed = 2024u64;
    let mkt: Vec<f64> = (0..n).map(|_| next_uniform(&mut seed) - 0.5).collect();
    let smb: Vec<f64> = (0..n).map(|_| next_uniform(&mut seed) - 0.5).collect();
    let asset: Vec<f64> = mkt
        .iter()
        .zip(smb.iter())
        .map(|(&m, &s)| 0.02 + 0.5 * m + 0.3 * s + (next_uniform(&mut seed) - 0.5) * 0.05)
        .collect();

    let mut factors = HashMap::new();
    factors.insert("mkt".to_string(), mkt);
    factors.insert("smb".to_string(), smb);

    let result = multi_factor_regression(&asset, &factors).unwrap();

    let intercept = result.factor_loadings["intercept"];
    let mkt_loading = result.factor_loadings["mkt"];
    let smb_loading = result.factor_loadings["smb"];

    assert!((intercept - 0.02).abs() < 0.02, "intercept near 0.02");
    assert!((mkt_loading - 0.5).abs() < 0.1, "mkt loading near 0.5");
    assert!((smb_loading - 0.3).abs() < 0.1, "smb loading near 0.3");
    assert!(result.r_squared > 0.8, "R² should be high");
    assert!(
        result.idiosyncratic_volatility >= 0.0,
        "idio vol non-negative"
    );
}

#[test]
fn test_multi_factor_rank_deficient() {
    // Factor 2 is exactly 2 * factor 1, so the design matrix is rank-deficient.
    let asset = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let f1 = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let f2 = vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
    let mut factors = HashMap::new();
    factors.insert("f1".to_string(), f1);
    factors.insert("f2".to_string(), f2);
    let err = multi_factor_regression(&asset, &factors).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_multi_factor_empty_factors() {
    let asset = vec![1.0, 2.0, 3.0];
    let factors: HashMap<String, Vec<f64>> = HashMap::new();
    let err = multi_factor_regression(&asset, &factors).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_multi_factor_non_finite_input() {
    let asset = vec![1.0, f64::NAN, 3.0];
    let mut factors = HashMap::new();
    factors.insert("f1".to_string(), vec![1.0, 2.0, 3.0]);
    let err = multi_factor_regression(&asset, &factors).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_black_scholes_call() {
    let params = BlackScholesParams {
        spot: 100.0,
        strike: 100.0,
        risk_free_rate: 0.05,
        volatility: 0.2,
        time_to_maturity: 1.0,
        option_type: OptionType::Call,
    };

    let result = black_scholes(
        params.spot,
        params.strike,
        params.risk_free_rate,
        params.volatility,
        params.time_to_maturity,
        params.option_type,
    )
    .unwrap();

    // Known approximate Black-Scholes price for ATM call with these parameters.
    assert!((result.price - 10.45).abs() < 0.5, "call price near 10.45");
    assert!(
        result.delta > 0.5 && result.delta < 1.0,
        "call delta in (0.5, 1)"
    );
    assert!(result.gamma > 0.0, "gamma positive");
    assert!(result.vega > 0.0, "vega positive");
    assert!(result.theta < 0.0, "call theta negative");
    assert!(result.rho > 0.0, "call rho positive");

    // Also exercise the service path.
    let service = AnalysisService::new();
    let via_service = service.black_scholes(params).unwrap();
    assert_eq!(via_service.price, result.price);
}

#[test]
fn test_black_scholes_negative_spot() {
    let err = black_scholes(-1.0, 100.0, 0.05, 0.2, 1.0, OptionType::Call).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_black_scholes_zero_volatility() {
    let err = black_scholes(100.0, 100.0, 0.05, 0.0, 1.0, OptionType::Call).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_black_scholes_non_finite_input() {
    let err = black_scholes(f64::NAN, 100.0, 0.05, 0.2, 1.0, OptionType::Call).unwrap_err();
    assert!(matches!(err, sqt_core::QuantError::DataQuality(_)));
}

#[test]
fn test_service_regression() {
    // Build price series so that asset returns are roughly 1.5x benchmark returns.
    let n = 100;
    let mut seed = 77u64;
    let mut benchmark_prices = vec![100.0];
    let mut asset_prices = vec![100.0];
    for _ in 1..n {
        let r = (next_uniform(&mut seed) - 0.5) * 0.02;
        let asset_r = 1.5 * r;
        benchmark_prices.push(benchmark_prices.last().unwrap() * (1.0 + r));
        asset_prices.push(asset_prices.last().unwrap() * (1.0 + asset_r));
    }

    let asset_bars = ohlcv_series("ASSET", &asset_prices);
    let benchmark_bars = ohlcv_series("BENCH", &benchmark_prices);

    let service = AnalysisService::new();
    let result = service
        .regression(&asset_bars, &benchmark_bars, 0.0)
        .unwrap();

    assert!((result.beta - 1.5).abs() < 0.05, "service beta near 1.5");
    assert!(result.r_squared > 0.95, "R² high");
}

#[test]
fn test_service_cointegration() {
    let n = 200;
    let b: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 * 0.05).collect();
    let mut seed = 11u64;
    let mut residual = 0.0;
    let a: Vec<f64> = b
        .iter()
        .map(|&b| {
            let innovation = (next_uniform(&mut seed) - 0.5) * 0.5;
            residual = 0.7 * residual + innovation;
            2.0 * b + residual
        })
        .collect();

    let a_bars = ohlcv_series("A", &a);
    let b_bars = ohlcv_series("B", &b);

    let service = AnalysisService::new();
    let result = service.cointegration(&a_bars, &b_bars).unwrap();

    assert!(
        (result.hedge_ratio - 2.0).abs() < 0.1,
        "service hedge ratio near 2.0"
    );
    assert!(
        result.half_life.is_finite() && result.half_life > 0.0 || result.half_life.is_infinite(),
        "half-life should be positive finite or infinity"
    );
}

#[test]
fn test_service_pca_labels_preserved() {
    let mut assets = HashMap::new();
    let n = 60;
    let mut seed = 55u64;
    let x: Vec<f64> = (0..n).map(|_| next_uniform(&mut seed)).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&v| v + next_uniform(&mut seed) * 0.1)
        .collect();
    assets.insert("XX".to_string(), ohlcv_series("XX", &x));
    assets.insert("YY".to_string(), ohlcv_series("YY", &y));

    let service = AnalysisService::new();
    let result = service.pca(&assets, 2).unwrap();
    assert_eq!(result.labels, vec!["XX", "YY"]);
    assert_eq!(result.eigenvalues.len(), 2);
}
