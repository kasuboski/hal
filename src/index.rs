//! # Vector Database Index Module for RAG
//!
//! This module provides the vector database infrastructure for the RAG pipeline,
//! responsible for storing and retrieving embedded content chunks and associated metadata.
//! It serves as the persistent storage layer that bridges content processing and search.
//!
//! ## Key Components
//!
//! - `Database`: Main interface for interacting with the LibSQL vector database
//! - `Website`: Represents metadata about an indexed website
//! - `IndexedChunk`: Represents a processed and indexed content chunk with its embedding
//!
//! ## Features
//!
//! - LibSQL-powered vector database with vector search capabilities
//! - Schema management for websites and chunks
//! - Transactional operations for data integrity
//! - Efficient embedding storage and retrieval
//! - Website and content metadata management
//! - Batch operations for indexing and updating content
//! - Reembedding utilities for updating vector representations
//!
//! ## Storage Model
//!
//! The module implements a two-level storage model:
//! 1. Websites: Store metadata about content sources
//! 2. Chunks: Store individual content segments with their embeddings and contextual information
//!
//! This design enables efficient vector search across content while maintaining
//! source attribution and metadata for retrieved results.

mod database;
pub mod error;
mod schema;

pub use database::Database;
pub use error::DbError;
use rig::embeddings::Embedding;

/// Represents a website in the index
#[derive(Debug, Clone)]
pub struct Website {
    /// ID of the website
    pub id: i64,

    /// URL of the website
    pub url: String,

    /// Domain of the website
    pub domain: String,

    /// Date of first indexing
    pub first_index_date: i64,

    /// Date of last indexing
    pub last_index_date: i64,

    /// Number of pages indexed
    pub page_count: i64,

    /// Status of the website
    pub status: String,
}

/// Represents a chunk in the index
#[derive(Debug, Clone)]
pub struct IndexedChunk {
    /// ID of the chunk
    pub id: i64,

    /// ID of the website
    pub website_id: i64,

    /// URL of the page
    pub url: String,

    /// Text of the chunk
    pub text: String,

    /// Context string for the chunk
    pub context: String,

    /// Embedding of the chunk
    pub embedding: Embedding,

    /// Position of the chunk in the document
    pub position: i64,

    /// Heading of the chunk
    pub heading: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_website_struct() {
        let website = Website {
            id: 1,
            url: "https://example.com".to_string(),
            domain: "example.com".to_string(),
            first_index_date: 1625097600, // 2021-07-01
            last_index_date: 1625097600,
            page_count: 10,
            status: "active".to_string(),
        };

        assert_eq!(website.id, 1);
        assert_eq!(website.url, "https://example.com");
        assert_eq!(website.domain, "example.com");
        assert_eq!(website.first_index_date, 1625097600);
        assert_eq!(website.last_index_date, 1625097600);
        assert_eq!(website.page_count, 10);
        assert_eq!(website.status, "active");
    }

    #[test]
    fn test_indexed_chunk_struct() {
        let chunk = IndexedChunk {
            id: 1,
            website_id: 1,
            url: "https://example.com/page".to_string(),
            text: "This is a test chunk".to_string(),
            context: "Context for the test chunk".to_string(),
            embedding: Embedding {
                document: "This is a test chunk".to_string(),
                vec: vec![0.1, 0.2, 0.3, 0.4],
            },
            position: 1,
            heading: Some("Test Heading".to_string()),
        };

        assert_eq!(chunk.id, 1);
        assert_eq!(chunk.website_id, 1);
        assert_eq!(chunk.url, "https://example.com/page");
        assert_eq!(chunk.text, "This is a test chunk");
        assert_eq!(chunk.context, "Context for the test chunk");
        assert_eq!(chunk.embedding.document, "This is a test chunk");
        assert_eq!(chunk.embedding.vec, vec![0.1, 0.2, 0.3, 0.4]);
        assert_eq!(chunk.position, 1);
        assert_eq!(chunk.heading, Some("Test Heading".to_string()));
    }

    #[test]
    fn test_indexed_chunk_without_heading() {
        let chunk = IndexedChunk {
            id: 2,
            website_id: 1,
            url: "https://example.com/page".to_string(),
            text: "This is another test chunk".to_string(),
            context: "Context for another test chunk".to_string(),
            embedding: Embedding {
                document: "This is another test chunk".to_string(),
                vec: vec![0.5, 0.6, 0.7, 0.8],
            },
            position: 2,
            heading: None,
        };

        assert_eq!(chunk.id, 2);
        assert_eq!(chunk.website_id, 1);
        assert_eq!(chunk.url, "https://example.com/page");
        assert_eq!(chunk.text, "This is another test chunk");
        assert_eq!(chunk.context, "Context for another test chunk");
        assert_eq!(chunk.embedding.document, "This is another test chunk");
        assert_eq!(chunk.embedding.vec, vec![0.5, 0.6, 0.7, 0.8]);
        assert_eq!(chunk.position, 2);
        assert_eq!(chunk.heading, None);
    }
}
