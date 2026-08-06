//! Audit writer.
//!
//! [`AuditWriter`] appends immutable records to an [`AuditStorage`] backend,
//! computing the record hash and linking each record to the previous one.

use std::sync::Arc;

use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use crate::hash::{compute_record_hash, GENESIS_HASH};
use crate::record::AuditRecord;
use crate::storage::AuditStorage;

/// Writes audit records to a storage backend.
#[derive(Clone)]
pub struct AuditWriter<S: AuditStorage> {
    storage: Arc<S>,
}

impl<S: AuditStorage> AuditWriter<S> {
    /// Creates a writer backed by the supplied storage.
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// Returns a clone of the underlying storage handle.
    pub fn storage(&self) -> Arc<S> {
        self.storage.clone()
    }

    /// Records a tool execution.
    ///
    /// The record is linked to the previous record's hash (or the genesis hash
    /// for the first record) and the new hash is computed deterministically.
    pub async fn record(
        &self,
        request_id: Uuid,
        tool_name: impl Into<String>,
        input: Value,
        output: Option<&Value>,
        status: impl Into<String>,
        error_message: Option<String>,
    ) -> crate::Result<AuditRecord> {
        let tool_name = tool_name.into();
        let status = status.into();
        let output_hash = output.map(crate::hash::hash_payload);
        let prev_record_hash = self.storage.last_hash().await?;
        let recorded_at = Utc::now();
        let id = Uuid::new_v4();

        let record_hash = compute_record_hash(
            &id,
            &request_id,
            &recorded_at,
            &tool_name,
            &input,
            output_hash.as_deref(),
            &status,
            error_message.as_deref(),
            &prev_record_hash,
        );

        let record = AuditRecord {
            id,
            request_id,
            recorded_at,
            tool_name,
            input,
            output_hash,
            status,
            error_message,
            prev_record_hash,
            record_hash,
        };

        self.storage.append(&record).await?;
        Ok(record)
    }

    /// Convenience method that records a successful execution.
    pub async fn record_ok(
        &self,
        request_id: Uuid,
        tool_name: impl Into<String>,
        input: Value,
        output: &Value,
    ) -> crate::Result<AuditRecord> {
        self.record(request_id, tool_name, input, Some(output), "ok", None)
            .await
    }

    /// Convenience method that records a failed execution.
    pub async fn record_error(
        &self,
        request_id: Uuid,
        tool_name: impl Into<String>,
        input: Value,
        error_message: String,
    ) -> crate::Result<AuditRecord> {
        self.record(
            request_id,
            tool_name,
            input,
            None::<&Value>,
            "error",
            Some(error_message),
        )
        .await
    }
}

impl<S: AuditStorage> AuditWriter<S> {
    /// Returns the genesis hash constant.
    pub fn genesis_hash() -> &'static str {
        GENESIS_HASH
    }
}
