//! # Search Error Types Module
//!
//! This module defines error types specific to the search component of the RAG pipeline.
//! It provides structured error handling for various failure modes during semantic search.
//!
//! ## Key Components
//!
//! - `SearchError`: Enum representing different types of search failures
//!
//! ## Features
//!
//! - Specialized error types for different search failure scenarios
//! - Database error handling through composition with `DbError`
//! - Embedding generation error handling
//! - Query processing error handling
//! - Result processing error handling
//! - Parameter validation error handling
//! - Conversion implementations for common error types
//!
//! The error types in this module help with debugging search issues and provide
//! useful information about where in the search pipeline a failure occurred,
//! enabling better error handling and user feedback.

use thiserror::Error;

use crate::index::DbError;

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

impl From<serde_json::Error> for SearchError {
    fn from(err: serde_json::Error) -> Self {
        SearchError::ResultProcessing(err.to_string())
    }
}

impl From<libsql::Error> for SearchError {
    fn from(err: libsql::Error) -> Self {
        SearchError::Database(DbError::Query(err.to_string()))
    }
}
