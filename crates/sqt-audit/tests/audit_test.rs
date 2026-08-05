//! Integration tests for the `sqt-audit` crate.

use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use sqt_audit::{
    AuditRecord, AuditReplayer, AuditStorage, AuditVerifier, AuditWriter, InMemoryStorage,
    ReplayDispatcher, VerificationResult,
};

fn build_storage() -> Arc<InMemoryStorage> {
    Arc::new(InMemoryStorage::new())
}

#[tokio::test]
async fn writer_links_records_in_a_chain() {
    let storage = build_storage();
    let writer = AuditWriter::new(storage.clone());

    let request_id = Uuid::new_v4();
    let first = writer
        .record_ok(request_id, "tool_a", json!({"x": 1}), &json!({"y": 2}))
        .await
        .expect("record succeeds");

    assert_eq!(first.prev_record_hash, sqt_audit::GENESIS_HASH);

    let second = writer
        .record_ok(request_id, "tool_b", json!({"x": 2}), &json!({"y": 4}))
        .await
        .expect("record succeeds");

    assert_eq!(second.prev_record_hash, first.record_hash);

    let records = storage.list().await.expect("list succeeds");
    assert_eq!(records.len(), 2);
}

#[tokio::test]
async fn verifier_detects_tampering() {
    let storage = build_storage();
    let writer = AuditWriter::new(storage.clone());

    let request_id = Uuid::new_v4();
    writer
        .record_ok(request_id, "tool_a", json!({"x": 1}), &json!({"y": 2}))
        .await
        .expect("record succeeds");

    // Append a record with an invalid record hash; the previous-hash link is correct,
    // but the record hash does not match the contents.
    let last_hash = storage.last_hash().await.expect("last hash");
    let tampered = AuditRecord::new(
        Uuid::new_v4(),
        request_id,
        chrono::Utc::now(),
        "tool_b",
        json!({"x": 2}),
        None,
        "ok",
        None,
        last_hash,
        "invalid_hash",
    );
    storage.append(&tampered).await.expect("append succeeds");

    let verifier = AuditVerifier::new(storage.clone());
    let result = verifier.verify().await.expect("verify runs");
    assert!(
        matches!(result, VerificationResult::Tampered { .. }),
        "expected tampered result"
    );
}

#[tokio::test]
async fn verifier_accepts_intact_chain() {
    let storage = build_storage();
    let writer = AuditWriter::new(storage.clone());

    let request_id = Uuid::new_v4();
    writer
        .record_ok(request_id, "tool_a", json!({"x": 1}), &json!({"y": 2}))
        .await
        .expect("record succeeds");
    writer
        .record_error(
            request_id,
            "tool_b",
            json!({"x": 2}),
            "something went wrong".to_string(),
        )
        .await
        .expect("record succeeds");

    let verifier = AuditVerifier::new(storage.clone());
    let result = verifier.verify().await.expect("verify runs");
    assert_eq!(result, VerificationResult::Ok);
}

struct EchoDispatcher;

#[async_trait::async_trait]
impl ReplayDispatcher for EchoDispatcher {
    async fn dispatch(
        &self,
        _tool_name: &str,
        input: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        Ok(input.clone())
    }
}

#[tokio::test]
async fn replay_detects_output_mismatch() {
    let storage = build_storage();
    let writer = AuditWriter::new(storage.clone());

    let request_id = Uuid::new_v4();
    writer
        .record_ok(request_id, "tool_a", json!({"x": 1}), &json!({"y": 2}))
        .await
        .expect("record succeeds");

    // EchoDispatcher returns the input, which hashes differently from the stored output.
    let replayer = AuditReplayer::new(storage.clone());
    let summary = replayer
        .replay(&EchoDispatcher)
        .await
        .expect("replay succeeds");

    assert_eq!(summary.total, 1);
    assert_eq!(summary.mismatches, 1);
    assert_eq!(summary.errors, 0);
}

#[tokio::test]
async fn replay_succeeds_when_output_matches() {
    let storage = build_storage();
    let writer = AuditWriter::new(storage.clone());

    let request_id = Uuid::new_v4();
    let input = json!({"x": 1});
    writer
        .record_ok(request_id, "tool_a", input.clone(), &input)
        .await
        .expect("record succeeds");

    let replayer = AuditReplayer::new(storage.clone());
    let summary = replayer
        .replay(&EchoDispatcher)
        .await
        .expect("replay succeeds");

    assert!(summary.is_consistent());
}

#[tokio::test]
async fn genesis_hash_is_used_for_empty_storage() {
    let storage = build_storage();
    let last = storage.last_hash().await.expect("last_hash succeeds");
    assert_eq!(last, sqt_audit::GENESIS_HASH);
}
