//! Shared error types and result alias for the `sqt-core` crate.
//!
//! All fallible public APIs in this crate return [`Result<T>`], which is a thin
//! alias for `std::result::Result<T, QuantError>`. [`QuantError`] enumerates the
//! high-level failure modes that callers are expected to handle.

use thiserror::Error;

/// The error type returned by fallible APIs in `sqt-core`.
#[derive(Error, Debug)]
pub enum QuantError {
    /// The caller supplied an invalid command or set of parameters.
    #[error("invalid command: {0}")]
    InvalidCommand(String),

    /// A requested data provider is not currently available.
    #[error("provider not available: {0}")]
    ProviderNotAvailable(String),

    /// A data quality issue was detected (for example, missing or corrupt data).
    #[error("data quality issue: {0}")]
    DataQuality(String),

    /// The requested resource or entity was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// An unexpected internal error.
    ///
    /// This wraps [`anyhow::Error`] so that internal failures can be converted
    /// ergonomically with the `?` operator while keeping the public error
    /// surface small.
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Result alias used by fallible APIs in `sqt-core`.
pub type Result<T> = std::result::Result<T, QuantError>;
