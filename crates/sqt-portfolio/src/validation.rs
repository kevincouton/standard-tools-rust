//! Shared validation helpers for portfolio optimizers.

use std::collections::HashSet;

use sqt_core::{QuantError, Result};

/// Maximum number of assets the portfolio optimizers will accept.
pub const MAX_PORTFOLIO_ASSETS: usize = 100;

/// Returns an error if `labels` contains duplicate entries or exceeds the asset limit.
pub fn validate_labels(labels: &[String]) -> Result<()> {
    if labels.len() > MAX_PORTFOLIO_ASSETS {
        return Err(QuantError::InvalidCommand(format!(
            "portfolio optimization supports at most {MAX_PORTFOLIO_ASSETS} assets; got {}",
            labels.len()
        )));
    }
    let mut seen = HashSet::with_capacity(labels.len());
    for label in labels {
        if !seen.insert(label) {
            return Err(QuantError::InvalidCommand(format!(
                "duplicate label `{label}`"
            )));
        }
    }
    Ok(())
}
