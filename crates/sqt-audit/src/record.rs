//! Audit record model.
//!
//! An [`AuditRecord`] represents a single immutable entry in the hash-chained
//! audit trail. Each record links to the previous record's hash so that the
//! integrity of the entire chain can be verified later.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// A single audit-trail record.
///
/// Records are ordered by insertion time and chained through
/// `prev_record_hash` / `record_hash`. The genesis record uses
/// [`GENESIS_HASH`](crate::hash::GENESIS_HASH) as its previous hash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditRecord {
    /// Stable record identifier.
    pub id: Uuid,
    /// Request/correlation identifier that groups related tool executions.
    pub request_id: Uuid,
    /// UTC timestamp when the record was created.
    pub recorded_at: DateTime<Utc>,
    /// Name of the tool that was invoked.
    pub tool_name: String,
    /// Tool input arguments.
    pub input: Value,
    /// Hash of the tool output (if any).
    pub output_hash: Option<String>,
    /// Execution status, e.g. `ok` or `error`.
    pub status: String,
    /// Error message when the tool failed.
    pub error_message: Option<String>,
    /// Hash of the previous record in the chain.
    pub prev_record_hash: String,
    /// Hash of this record itself.
    pub record_hash: String,
}

impl AuditRecord {
    /// Convenience constructor for tests and in-memory storage.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        request_id: Uuid,
        recorded_at: DateTime<Utc>,
        tool_name: impl Into<String>,
        input: Value,
        output_hash: Option<String>,
        status: impl Into<String>,
        error_message: Option<String>,
        prev_record_hash: impl Into<String>,
        record_hash: impl Into<String>,
    ) -> Self {
        Self {
            id,
            request_id,
            recorded_at,
            tool_name: tool_name.into(),
            input,
            output_hash,
            status: status.into(),
            error_message,
            prev_record_hash: prev_record_hash.into(),
            record_hash: record_hash.into(),
        }
    }
}
