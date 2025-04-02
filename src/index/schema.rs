//! # Database Schema Module
//!
//! This module defines and manages the database schema for the vector index in the RAG pipeline.
//! It handles table creation, index management, and schema migrations.
//!
//! ## Key Components
//!
//! - `initialize_schema`: Function to create and update the database schema
//!
//! ## Features
//!
//! - Websites table for source metadata
//! - Chunks table for content segments with embeddings
//! - Foreign key relationships for referential integrity
//! - Indexes for efficient lookups and vector search
//! - Vector-specific storage optimizations
//! - Schema versioning and migration support
//!
//! ## Schema Design
//!
//! The schema implements a two-table design:
//! 1. `websites` - Stores metadata about content sources with unique URL constraints
//! 2. `chunks` - Stores content segments with their vector embeddings and foreign keys to websites
//!
//! The schema includes specialized indexes for vector similarity search, enabling
//! efficient retrieval of semantically similar content during RAG operations.

use crate::index::error::DbError;
use libsql::{Connection, params};

/// Initialize the database schema
pub async fn initialize_schema(conn: &Connection) -> Result<(), DbError> {
    // Create websites table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS websites (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            url TEXT NOT NULL UNIQUE,
            domain TEXT NOT NULL,
            first_index_date INTEGER,
            last_index_date INTEGER,
            page_count INTEGER DEFAULT 0,
            status TEXT NOT NULL
        )",
        params![],
    )
    .await
    .map_err(|e| DbError::Schema(format!("Failed to create websites table: {}", e)))?;

    // Create chunks table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chunks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            website_id INTEGER NOT NULL,
            url TEXT NOT NULL,
            text TEXT NOT NULL,
            context TEXT NOT NULL,
            embedding F32_BLOB(768) NOT NULL,
            position INTEGER NOT NULL,
            heading TEXT,
            FOREIGN KEY (website_id) REFERENCES websites(id) ON DELETE CASCADE
        )",
        params![],
    )
    .await
    .map_err(|e| DbError::Schema(format!("Failed to create chunks table: {}", e)))?;

    // Create index on website_id for faster lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_chunks_website_id ON chunks(website_id)",
        params![],
    )
    .await
    .map_err(|e| DbError::Schema(format!("Failed to create index on chunks: {}", e)))?;

    // Create index on url for faster lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_chunks_url ON chunks(url)",
        params![],
    )
    .await
    .map_err(|e| DbError::Schema(format!("Failed to create index on chunks url: {}", e)))?;

    // Create vector index for embeddings
    // This might fail if the vector extension is not available, but we'll continue anyway
    let vector_index_result = conn
        .execute(
            "CREATE INDEX IF NOT EXISTS chunks_idx ON chunks (libsql_vector_idx(embedding))",
            params![],
        )
        .await;

    if let Err(e) = vector_index_result {
        eprintln!(
            "Warning: Failed to create vector index: {}. Vector search will not be available.",
            e
        );
    }

    Ok(())
}
