//! Error types for the search module

use crate::index::error::DbError;
use thiserror::Error;

/// Errors that can occur during search operations
#[derive(Debug, Error)]
pub enum SearchError {
    /// Error occurred during database operations
    #[error("Database error: {0}")]
    Database(#[from] DbError),

    /// Error occurred during embedding generation
    #[error("Embedding error: {0}")]
    Embedding(String),

    /// Error occurred during query processing
    #[error("Query error: {0}")]
    Query(String),

    /// Error occurred during result processing
    #[error("Result processing error: {0}")]
    ResultProcessing(String),

    /// Invalid search parameters
    #[error("Invalid search parameters: {0}")]
    InvalidParameters(String),
}

impl From<libsql::Error> for SearchError {
    fn from(err: libsql::Error) -> Self {
        SearchError::Database(DbError::Query(err.to_string()))
    }
}

impl From<serde_json::Error> for SearchError {
    fn from(err: serde_json::Error) -> Self {
        SearchError::ResultProcessing(err.to_string())
    }
}
