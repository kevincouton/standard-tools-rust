//! Audit replay.
//!
//! [`AuditReplayer`] walks the audit chain and re-executes each record through
//! a caller-supplied [`ReplayDispatcher`]. The output of the replayed execution
//! is hashed and compared with the stored `output_hash` so that callers can
//! detect deterministic drift or tampering.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::hash::hash_payload;
use crate::storage::AuditStorage;

/// Dispatch interface supplied by the caller.
///
/// Implementors receive the original tool name and input and must return the
/// replayed output. The `sqt-api` crate typically implements this by delegating
/// to `sqt_agent::ToolDispatcher`.
#[async_trait]
pub trait ReplayDispatcher: Send + Sync {
    /// Re-executes a tool invocation and returns its output.
    async fn dispatch(&self, tool_name: &str, input: &Value) -> Result<Value, String>;
}

/// Replays an audit chain and compares outputs.
#[derive(Clone)]
pub struct AuditReplayer<S: AuditStorage> {
    storage: Arc<S>,
}

impl<S: AuditStorage> AuditReplayer<S> {
    /// Creates a replayer backed by the supplied storage.
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// Replays every record in the chain.
    ///
    /// Returns the number of records replayed and the number of mismatches.
    /// A mismatch occurs when the recomputed output hash differs from the
    /// stored hash.
    pub async fn replay<D: ReplayDispatcher>(
        &self,
        dispatcher: &D,
    ) -> crate::Result<ReplaySummary> {
        let records = self.storage.list().await?;
        let mut total = 0;
        let mut mismatches = 0;
        let mut errors = 0;

        for record in &records {
            total += 1;

            match dispatcher.dispatch(&record.tool_name, &record.input).await {
                Ok(output) => {
                    let output_hash = hash_payload(&output);
                    if Some(&output_hash) != record.output_hash.as_ref() {
                        mismatches += 1;
                    }
                }
                Err(_) => {
                    errors += 1;
                }
            }
        }

        Ok(ReplaySummary {
            total,
            mismatches,
            errors,
        })
    }
}

/// Summary of a replay run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplaySummary {
    /// Number of records replayed.
    pub total: usize,
    /// Number of records whose output hash differed from the stored hash.
    pub mismatches: usize,
    /// Number of records that could not be replayed due to dispatcher errors.
    pub errors: usize,
}

impl ReplaySummary {
    /// Returns true when every replayed record matched its stored output hash.
    pub fn is_consistent(&self) -> bool {
        self.mismatches == 0 && self.errors == 0
    }
}
