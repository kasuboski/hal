//! Search implementation for RAG functionality

use super::error::SearchError;
use crate::gemini::prelude::Content;
use crate::gemini::Client;
use crate::index::Database;
use serde::{Deserialize, Serialize};

/// Options for search queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Maximum number of results to return
    pub limit: usize,

    /// Filter by source domain
    pub source_filter: Option<String>,

    /// Filter by date range (start_timestamp, end_timestamp)
    pub date_range: Option<(i64, i64)>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            source_filter: None,
            date_range: None,
        }
    }
}

/// Search result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// ID of the chunk
    pub chunk_id: i64,

    /// Text content of the chunk
    pub text: String,

    /// Summary of the chunk
    pub summary: String,

    /// Context information
    pub context: String,

    /// URL of the source page
    pub url: String,

    /// URL of the source website
    pub website_url: String,

    /// Domain of the source website
    pub website_domain: String,

    /// Similarity score (constant 1.0 for vector search results)
    pub score: f64,
}

/// Search the index with the given query and options
pub async fn search_index(
    db: &Database,
    query: &str,
    options: SearchOptions,
) -> Result<Vec<SearchResult>, SearchError> {
    // Create a client to use for embedding
    let api_key = std::env::var("GEMINI_API_KEY").map_err(|_| {
        SearchError::Embedding("GEMINI_API_KEY environment variable not set".to_string())
    })?;
    let client = Client::with_api_key_rate_limited(api_key);

    // Call the internal function with the client
    search_index_with_client(db, &client, query, options).await
}

/// Search the index with the given query, options, and client
pub async fn search_index_with_client(
    db: &Database,
    client: &Client,
    query: &str,
    options: SearchOptions,
) -> Result<Vec<SearchResult>, SearchError> {
    // Generate embedding for query
    let content = Content::new().with_text(query);
    let query_embedding = client
        .models()
        .embed_content("text-embedding-004", content)
        .await
        .map_err(|e| SearchError::Embedding(format!("Failed to generate embedding: {}", e)))?;

    // Convert embedding to binary blob for vector search
    let embedding_values = &query_embedding.embedding.values;
    let embedding_blob: Vec<u8> = embedding_values
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect();

    // Perform vector search
    vector_search(db, &embedding_blob, &options).await
}

/// Search using the vector_top_k function
async fn vector_search(
    db: &Database,
    embedding_blob: &[u8],
    options: &SearchOptions,
) -> Result<Vec<SearchResult>, SearchError> {
    // Build SQL query using vector_top_k for proper vector similarity search
    let mut sql = String::from(
        "SELECT 
            c.id, c.text, c.summary, c.context, c.url,
            w.url as website_url, w.domain as website_domain,
            1.0 as score
        FROM vector_top_k('chunks_idx', ?, ?) as v
        JOIN chunks c ON c.rowid = v.id
        JOIN websites w ON c.website_id = w.id",
    );

    // Add source filter if specified
    if options.source_filter.is_some() {
        sql.push_str(" WHERE w.domain LIKE ?");
    } else {
        sql.push_str(" WHERE 1=1"); // Always true condition to simplify adding more conditions
    }

    // Add date range filter if specified
    if options.date_range.is_some() {
        sql.push_str(" AND w.last_index_date >= ? AND w.last_index_date <= ?");
    }

    // Prepare query parameters
    let mut params: Vec<libsql::Value> = Vec::new();
    params.push(libsql::Value::Blob(embedding_blob.to_vec())); // Query vector for vector_top_k
    params.push(libsql::Value::from(options.limit as i64)); // k value for vector_top_k

    if let Some(source) = &options.source_filter {
        params.push(format!("%{}%", source).into());
    }

    if let Some((start, end)) = options.date_range {
        params.push(start.into());
        params.push(end.into());
    }

    // Execute query
    let rows = db.execute_query(&sql, params).await?;

    // Process results
    process_results(rows).await
}

/// Process the results from a query into SearchResult objects
async fn process_results(mut rows: libsql::Rows) -> Result<Vec<SearchResult>, SearchError> {
    let mut results = Vec::new();
    while let Ok(Some(row)) = rows.next().await {
        results.push(SearchResult {
            chunk_id: row.get(0).map_err(|e| {
                SearchError::ResultProcessing(format!("Failed to get chunk_id: {}", e))
            })?,
            text: row
                .get(1)
                .map_err(|e| SearchError::ResultProcessing(format!("Failed to get text: {}", e)))?,
            summary: row.get(2).map_err(|e| {
                SearchError::ResultProcessing(format!("Failed to get summary: {}", e))
            })?,
            context: row.get(3).map_err(|e| {
                SearchError::ResultProcessing(format!("Failed to get context: {}", e))
            })?,
            url: row
                .get(4)
                .map_err(|e| SearchError::ResultProcessing(format!("Failed to get url: {}", e)))?,
            website_url: row.get(5).map_err(|e| {
                SearchError::ResultProcessing(format!("Failed to get website_url: {}", e))
            })?,
            website_domain: row.get(6).map_err(|e| {
                SearchError::ResultProcessing(format!("Failed to get website_domain: {}", e))
            })?,
            score: row.get(7).map_err(|e| {
                SearchError::ResultProcessing(format!("Failed to get score: {}", e))
            })?,
        });
    }

    Ok(results)
}
