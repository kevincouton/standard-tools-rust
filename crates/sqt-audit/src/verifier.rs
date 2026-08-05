//! Audit-chain verifier.
//!
//! [`AuditVerifier`] walks the stored records and checks that:
//!
//! 1. The first record links to the genesis hash.
//! 2. Every subsequent record links to the previous record's hash.
//! 3. Each record's `record_hash` matches the recomputed hash of its contents.

use std::sync::Arc;

use sqt_core::{QuantError, Result};

use crate::hash::{compute_record_hash, GENESIS_HASH};
use crate::storage::AuditStorage;

/// Result of a chain verification.
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationResult {
    /// The chain is intact.
    Ok,
    /// The chain has been tampered with.
    Tampered {
        /// Index of the record that failed verification.
        index: usize,
        /// Description of the failure.
        reason: String,
    },
}

/// Verifies the integrity of an audit chain.
#[derive(Clone)]
pub struct AuditVerifier<S: AuditStorage> {
    storage: Arc<S>,
}

impl<S: AuditStorage> AuditVerifier<S> {
    /// Creates a verifier backed by the supplied storage.
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// Verifies the entire stored chain.
    pub async fn verify(&self) -> Result<VerificationResult> {
        let records = self.storage.list().await?;

        for (index, record) in records.iter().enumerate() {
            let expected_prev = if index == 0 {
                GENESIS_HASH.to_string()
            } else {
                records[index - 1].record_hash.clone()
            };

            if record.prev_record_hash != expected_prev {
                return Ok(VerificationResult::Tampered {
                    index,
                    reason: format!(
                        "expected prev_record_hash {expected_prev}, got {}",
                        record.prev_record_hash
                    ),
                });
            }

            let expected_hash = compute_record_hash(
                &record.id,
                &record.request_id,
                &record.recorded_at,
                &record.tool_name,
                &record.input,
                record.output_hash.as_deref(),
                &record.status,
                record.error_message.as_deref(),
                &record.prev_record_hash,
            );

            if record.record_hash != expected_hash {
                return Ok(VerificationResult::Tampered {
                    index,
                    reason: format!(
                        "expected record_hash {expected_hash}, got {}",
                        record.record_hash
                    ),
                });
            }
        }

        Ok(VerificationResult::Ok)
    }

    /// Verifies the chain and returns an error if tampered.
    pub async fn verify_strict(&self) -> Result<()> {
        match self.verify().await? {
            VerificationResult::Ok => Ok(()),
            VerificationResult::Tampered { index, reason } => Err(QuantError::DataQuality(
                format!("audit chain tampered at index {index}: {reason}"),
            )),
        }
    }
}
