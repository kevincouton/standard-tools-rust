//! Hash-chained audit trail for Standard-Tools.
//!
//! The `sqt-audit` crate provides immutable, cryptographically chained audit
//! records. Records can be persisted in memory (for tests) or in PostgreSQL
//! (for production). The chain can be verified for integrity and replayed
//! against a dispatcher to confirm deterministic outputs.
//!
//! # Example
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use sqt_audit::{AuditWriter, InMemoryStorage};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let storage = Arc::new(InMemoryStorage::new());
//! let writer = AuditWriter::new(storage.clone());
//! let record = writer.record_ok(
//!     uuid::Uuid::new_v4(),
//!     "black_scholes",
//!     serde_json::json!({ "spot": 100.0 }),
//!     &serde_json::json!({ "price": 10.0 }),
//! ).await?;
//! # Ok(())
//! # }
//! ```

pub mod hash;
pub mod record;
pub mod replay;
pub mod storage;
pub mod verifier;
pub mod writer;

pub use hash::{compute_record_hash, GENESIS_HASH};
pub use record::AuditRecord;
pub use replay::{AuditReplayer, ReplayDispatcher, ReplaySummary};
pub use storage::{AuditStorage, InMemoryStorage, SqlxStorage};
pub use verifier::{AuditVerifier, VerificationResult};
pub use writer::AuditWriter;

/// Convenience `Result` alias used by this crate.
pub type Result<T> = sqt_core::Result<T>;
