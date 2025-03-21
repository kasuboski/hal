//! # Semantic Search Module for RAG
//!
//! This module provides the vector similarity search capabilities for the RAG pipeline,
//! enabling retrieval of relevant content based on semantic similarity rather than
//! keyword matching. It forms the "retrieval" part of Retrieval Augmented Generation.
//!
//! ## Key Components
//!
//! - `SearchSystem`: Main interface for performing semantic searches
//! - `SearchOptions`: Configuration for filtering and limiting search results
//! - `SearchResult`: Represents a retrieved document with its metadata
//!
//! ## Features
//!
//! - Vector similarity search using LibSQL's vector extensions
//! - Filtering by source, date, and other metadata
//! - Relevance ranking based on embedding similarity
//! - Context preparation for RAG prompt construction
//! - Integration with LLM for answer generation from retrieved content
//! - Efficient query embedding generation
//!
//! ## Search Process
//!
//! 1. Convert the user query to an embedding vector
//! 2. Perform vector similarity search against the indexed embeddings
//! 3. Apply metadata filters (source, date, etc.)
//! 4. Retrieve and rank the most relevant content chunks
//! 5. Prepare context for LLM consumption
//! 6. Generate a response using the retrieved context
//!
//! This module bridges the gap between the vector database and the LLM,
//! enabling knowledge augmentation through efficient semantic retrieval.

mod error;
mod search_impl;

pub use error::SearchError;
pub use search_impl::{
    generate_answer_with_rag, prepare_rag_context, search_index, search_index_with_client,
    SearchOptions, SearchResult,
};

/// Re-export types needed for the search API
pub use crate::index::{Database, IndexedChunk, Website};

/// Search system for RAG
///
/// This module provides functionality to search the indexed content
/// using vector similarity search and filtering options.
pub struct SearchSystem<C, E>
where
    C: rig::completion::CompletionModel,
    E: rig::embeddings::EmbeddingModel,
{
    db: Database,
    client: crate::model::Client<C, E>,
}

impl<C, E> SearchSystem<C, E>
where
    C: rig::completion::CompletionModel,
    E: rig::embeddings::EmbeddingModel,
{
    /// Create a new search system with the given database
    pub fn new(db: Database, client: crate::model::Client<C, E>) -> Self {
        Self { db, client }
    }

    /// Search the index with the given query and options
    pub async fn search(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError> {
        search_index_with_client(&self.db, &self.client, query, options).await
    }

    /// Get the database reference
    pub fn database(&self) -> &Database {
        &self.db
    }

    /// Get the client reference
    pub fn client(&self) -> &crate::model::Client<C, E> {
        &self.client
    }
}

#[cfg(test)]
mod tests {

    use crate::search::search_impl::SearchOptions;

    #[test]
    fn test_search_options() {
        let options = SearchOptions {
            limit: 10,
            source_filter: Some("example.com".to_string()),
            date_range: Some((1000, 2000)),
        };

        assert_eq!(options.limit, 10);
        assert_eq!(options.source_filter.as_deref().unwrap(), "example.com");
        assert_eq!(options.date_range.unwrap(), (1000, 2000));
    }

    #[test]
    fn test_search_options_default() {
        let options = SearchOptions::default();

        assert_eq!(options.limit, 10);
        assert!(options.source_filter.is_none());
        assert!(options.date_range.is_none());
    }

    // Note: We're skipping the SearchSystem test as it requires a real Database instance
    // A more comprehensive test would use a mock database
}
