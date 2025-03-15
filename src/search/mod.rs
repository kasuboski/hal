//! Search module for RAG functionality
//!
//! This module provides search capabilities for the indexed content,
//! including vector search for semantic similarity.

mod error;
mod search;

pub use error::SearchError;
pub use search::{search_index, search_index_with_client, SearchOptions, SearchResult};

/// Re-export types needed for the search API
pub use crate::index::{Database, IndexedChunk, Website};

/// Search system for RAG
///
/// This module provides functionality to search the indexed content
/// using vector similarity search and filtering options.
pub struct SearchSystem {
    db: Database,
    client: crate::gemini::Client,
}

impl SearchSystem {
    /// Create a new search system with the given database
    pub fn new(db: Database, client: crate::gemini::Client) -> Self {
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
    pub fn client(&self) -> &crate::gemini::Client {
        &self.client
    }
}

#[cfg(test)]
mod tests {

    use crate::search::search::SearchOptions;

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
