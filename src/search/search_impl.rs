//! Search implementation for RAG functionality

use super::error::SearchError;
use crate::index::Database;
use crate::model::{Client, EmbeddingConversion};
use rig::{
    agent::AgentBuilder,
    completion::{CompletionModel, Prompt},
    embeddings::EmbeddingModel,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing::{debug, trace};

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

    /// Context information
    pub context: String,

    /// URL of the source page
    pub url: String,

    /// URL of the source website
    pub website_url: String,

    /// Domain of the source website
    pub website_domain: String,
}

/// Search the index with the given query and options
pub async fn search_index<C, E>(
    db: &Database,
    client: &Client<C, E>,
    query: &str,
    options: SearchOptions,
) -> Result<Vec<SearchResult>, SearchError>
where
    C: CompletionModel,
    E: EmbeddingModel,
{
    // Call the internal function with the client
    search_index_with_client(db, client, query, options).await
}

/// Search the index with the given query, options, and client
#[instrument(skip(db, client))]
pub async fn search_index_with_client<C, E>(
    db: &Database,
    client: &Client<C, E>,
    query: &str,
    options: SearchOptions,
) -> Result<Vec<SearchResult>, SearchError>
where
    C: CompletionModel,
    E: EmbeddingModel,
{
    // Generate embedding for query
    let query_embedding = client
        .embedding()
        .embed_text(query)
        .await
        .map_err(|e| SearchError::Embedding(format!("Failed to generate embedding: {}", e)))?;

    // Convert embedding to binary blob for vector search
    let embedding_blob = query_embedding.to_binary();

    // Perform vector search
    vector_search(db, &embedding_blob, &options).await
}

/// Search using the vector_top_k function
#[instrument(skip(db))]
async fn vector_search(
    db: &Database,
    embedding_blob: &[u8],
    options: &SearchOptions,
) -> Result<Vec<SearchResult>, SearchError> {
    // Build SQL query using vector_top_k for proper vector similarity search
    let mut sql = String::from(
        "SELECT
            c.id, c.text, c.context, c.url,
            w.url as website_url, w.domain as website_domain
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
            context: row.get(2).map_err(|e| {
                SearchError::ResultProcessing(format!("Failed to get context: {}", e))
            })?,
            url: row
                .get(3)
                .map_err(|e| SearchError::ResultProcessing(format!("Failed to get url: {}", e)))?,
            website_url: row.get(4).map_err(|e| {
                SearchError::ResultProcessing(format!("Failed to get website_url: {}", e))
            })?,
            website_domain: row.get(5).map_err(|e| {
                SearchError::ResultProcessing(format!("Failed to get website_domain: {}", e))
            })?,
        });
    }

    Ok(results)
}

/// Generate an answer using RAG
#[instrument(skip(client))]
pub async fn generate_answer_with_rag<C, E>(
    client: &crate::model::Client<C, E>,
    query: &str,
    context: &str,
    _model: &str,
) -> anyhow::Result<String>
where
    C: CompletionModel,
    E: EmbeddingModel,
{
    debug!("Generating answer for query of length {}", query.len());

    let completion = client.completion().clone();
    let agent = AgentBuilder::new(completion)
        .preamble("You are a helpful assistant that answers questions based on the provided context. \
        Use only the information from the context to answer the question. \
        If the context doesn't contain enough information to answer the question fully, \
        acknowledge the limitations and provide the best answer possible with the available information. \
        Be concise and accurate.\n")
        .build();

    // Create user prompt with context and query
    let user_prompt = format!("Context:\n{}\n\nQuestion: {}\n\nAnswer:", context, query);

    let answer = agent
        .prompt(user_prompt)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to generate answer: {}", e))?;

    trace!("Generated answer of length {}", answer.len());
    Ok(answer)
}

/// Prepare context from search results for RAG
pub fn prepare_rag_context(results: &[SearchResult]) -> String {
    let mut context = String::new();

    for (i, result) in results.iter().enumerate() {
        context.push_str(&format!("Source {}:\n", i + 1));
        context.push_str(&format!("URL: {}\n", result.url));
        context.push_str(&format!("Context: {}\n", result.context));
        context.push_str(&format!("Content: {}\n\n", result.text));
    }

    context
}
