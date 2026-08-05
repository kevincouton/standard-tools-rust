//! Small internal math helpers shared across `sqt-analysis`.

use sqt_core::{QuantError, Result};

/// Arithmetic mean of a non-empty slice.
pub(crate) fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

/// Validates that every value in the slice is finite.
///
/// Returns [`QuantError::DataQuality`] if any value is `NaN` or infinite.
pub(crate) fn validate_finite(values: &[f64]) -> Result<()> {
    if values.iter().any(|&v| !v.is_finite()) {
        return Err(QuantError::DataQuality(
            "input contains non-finite (NaN or infinite) values".to_string(),
        ));
    }
    Ok(())
}
