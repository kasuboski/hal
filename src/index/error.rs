//! # Database Error Types Module
//!
//! This module defines error types specific to the vector database component of the RAG pipeline.
//! It provides structured error handling for various failure modes during database operations.
//!
//! ## Key Components
//!
//! - `DbError`: Enum representing different types of database operation failures
//!
//! ## Features
//!
//! - Specialized error types for different database failure scenarios
//! - LibSQL-specific error handling
//! - Schema management error handling
//! - Data integrity and validation errors
//! - Connection management errors
//! - Transaction error handling
//! - Integration with the crate's main error type for consistent error propagation
//!
//! The error types in this module provide detailed information about database failures,
//! enabling proper error handling, debugging, and user feedback throughout the
//! RAG pipeline.

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
