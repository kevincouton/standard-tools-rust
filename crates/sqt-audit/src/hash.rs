//! Hash-chaining utilities for the audit trail.
//!
//! Records are hashed using SHA-256 over a canonical JSON representation so
//! that any tampering with the chain can be detected.

use sha2::{Digest, Sha256};

/// Hash used for the genesis (first) record.
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Computes the SHA-256 hash of a canonical JSON payload.
///
/// The payload is compact JSON with sorted keys, giving a deterministic hash
/// for the same logical content.
pub fn hash_payload(payload: &serde_json::Value) -> String {
    let canonical = serde_json::to_string(payload).expect("JSON serialization is infallible");
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

/// Builds the payload used to compute a record's hash.
///
/// The payload includes the previous record hash so that the chain cannot be
/// reordered or truncated without detection.
#[allow(clippy::too_many_arguments)]
pub fn record_hash_payload(
    id: &uuid::Uuid,
    request_id: &uuid::Uuid,
    recorded_at: &chrono::DateTime<chrono::Utc>,
    tool_name: &str,
    input: &serde_json::Value,
    output_hash: Option<&str>,
    status: &str,
    error_message: Option<&str>,
    prev_record_hash: &str,
) -> serde_json::Value {
    serde_json::json!({
        "id": id.to_string(),
        "request_id": request_id.to_string(),
        "recorded_at": recorded_at.to_rfc3339(),
        "tool_name": tool_name,
        "input": input,
        "output_hash": output_hash,
        "status": status,
        "error_message": error_message,
        "prev_record_hash": prev_record_hash,
    })
}

/// Computes the record hash for the supplied fields.
#[allow(clippy::too_many_arguments)]
pub fn compute_record_hash(
    id: &uuid::Uuid,
    request_id: &uuid::Uuid,
    recorded_at: &chrono::DateTime<chrono::Utc>,
    tool_name: &str,
    input: &serde_json::Value,
    output_hash: Option<&str>,
    status: &str,
    error_message: Option<&str>,
    prev_record_hash: &str,
) -> String {
    let payload = record_hash_payload(
        id,
        request_id,
        recorded_at,
        tool_name,
        input,
        output_hash,
        status,
        error_message,
        prev_record_hash,
    );
    hash_payload(&payload)
}

// Minimal hex encoder to avoid an extra dependency. SHA-256 produces 32 bytes.
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
    }
}
