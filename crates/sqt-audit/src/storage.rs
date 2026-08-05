//! Storage backends for the audit trail.
//!
//! The [`AuditStorage`] trait abstracts over persistence so that the audit
//! crate can be tested in memory and deployed with PostgreSQL in production.

use async_trait::async_trait;
use sqlx::{PgPool, Row};
use sqt_core::{QuantError, Result};

use crate::record::AuditRecord;

/// Abstract storage for audit records.
#[async_trait]
pub trait AuditStorage: Send + Sync {
    /// Returns the hash of the most recent record, or the genesis hash if the
    /// trail is empty.
    async fn last_hash(&self) -> Result<String>;

    /// Appends a record to the trail.
    async fn append(&self, record: &AuditRecord) -> Result<()>;

    /// Returns all records in insertion order.
    async fn list(&self) -> Result<Vec<AuditRecord>>;

    /// Returns the number of records in the trail.
    async fn len(&self) -> Result<usize>;

    /// Returns true if the trail contains no records.
    async fn is_empty(&self) -> Result<bool> {
        Ok(self.len().await? == 0)
    }
}

/// In-memory storage intended for tests and local development.
#[derive(Default, Debug, Clone)]
pub struct InMemoryStorage {
    records: std::sync::Arc<tokio::sync::Mutex<Vec<AuditRecord>>>,
}

impl InMemoryStorage {
    /// Creates a new empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AuditStorage for InMemoryStorage {
    async fn last_hash(&self) -> Result<String> {
        let records = self.records.lock().await;
        Ok(records
            .last()
            .map(|r| r.record_hash.clone())
            .unwrap_or_else(|| crate::hash::GENESIS_HASH.to_string()))
    }

    async fn append(&self, record: &AuditRecord) -> Result<()> {
        let mut records = self.records.lock().await;
        records.push(record.clone());
        Ok(())
    }

    async fn list(&self) -> Result<Vec<AuditRecord>> {
        let records = self.records.lock().await;
        Ok(records.clone())
    }

    async fn len(&self) -> Result<usize> {
        let records = self.records.lock().await;
        Ok(records.len())
    }
}

/// PostgreSQL-backed storage using SQLx.
#[derive(Debug, Clone)]
pub struct SqlxStorage {
    pool: PgPool,
}

impl SqlxStorage {
    /// Creates a new PostgreSQL-backed store from an existing pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creates the audit table if it does not already exist.
    pub async fn migrate(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_records (
                id UUID PRIMARY KEY,
                request_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL,
                tool_name TEXT NOT NULL,
                input JSONB NOT NULL,
                output_hash TEXT,
                status TEXT NOT NULL,
                error_message TEXT,
                prev_record_hash TEXT NOT NULL,
                record_hash TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| QuantError::Internal(anyhow::Error::new(e)))?;
        Ok(())
    }
}

#[async_trait]
impl AuditStorage for SqlxStorage {
    async fn last_hash(&self) -> Result<String> {
        let row =
            sqlx::query("SELECT record_hash FROM audit_records ORDER BY recorded_at DESC LIMIT 1")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| QuantError::Internal(anyhow::Error::new(e)))?;

        Ok(row
            .map(|r| r.get::<String, _>("record_hash"))
            .unwrap_or_else(|| crate::hash::GENESIS_HASH.to_string()))
    }

    async fn append(&self, record: &AuditRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO audit_records
                (id, request_id, recorded_at, tool_name, input, output_hash, status, error_message, prev_record_hash, record_hash)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(record.id)
        .bind(record.request_id)
        .bind(record.recorded_at)
        .bind(&record.tool_name)
        .bind(&record.input)
        .bind(&record.output_hash)
        .bind(&record.status)
        .bind(&record.error_message)
        .bind(&record.prev_record_hash)
        .bind(&record.record_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| QuantError::Internal(anyhow::Error::new(e)))?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<AuditRecord>> {
        sqlx::query_as::<_, AuditRecord>("SELECT * FROM audit_records ORDER BY recorded_at ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| QuantError::Internal(anyhow::Error::new(e)))
    }

    async fn len(&self) -> Result<usize> {
        let row = sqlx::query("SELECT COUNT(*) AS cnt FROM audit_records")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| QuantError::Internal(anyhow::Error::new(e)))?;
        let count: i64 = row.get("cnt");
        Ok(count as usize)
    }
}
