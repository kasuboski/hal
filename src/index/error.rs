//! Error types for the index module

use crate::error::Error as CrateError;
use thiserror::Error;

/// Error type for database operations
#[derive(Debug, Error)]
pub enum DbError {
    /// LibSQL error
    #[error("LibSQL error: {0}")]
    LibSql(#[from] libsql::Error),

    /// SQL query error
    #[error("SQL query error: {0}")]
    Query(String),

    /// Schema error
    #[error("Schema error: {0}")]
    Schema(String),

    /// Data error
    #[error("Data error: {0}")]
    Data(String),

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Transaction error
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// Other errors
    #[error("{0}")]
    Other(String),
}

impl From<DbError> for CrateError {
    fn from(err: DbError) -> Self {
        CrateError::Database(err.to_string())
    }
}
