//! Shared validation helpers for portfolio optimizers.

use std::collections::HashSet;

use sqt_core::{QuantError, Result};

/// Returns an error if `labels` contains duplicate entries.
pub fn check_unique_labels(labels: &[String]) -> Result<()> {
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
