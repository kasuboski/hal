//! Content processor module for RAG
//!
//! This module provides functionality for processing content,
//! including chunking, embedding generation, and LLM integration.

mod chunking;
mod config;
mod embedding;
mod error;
mod llm_integration;

pub use chunking::chunk_markdown;
pub use config::{ChunkOptions, ProcessorConfig};
pub use embedding::generate_embedding;
pub use error::ProcessError;
pub use llm_integration::{generate_context_string, generate_summary};

use crate::crawler::CrawledPage;
use crate::gemini::Client;
use futures::future;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, info};

/// Represents a processed chunk with its embedding, summary, and context
#[derive(Debug, Clone)]
pub struct ProcessedChunk {
    /// The text of the chunk
    pub text: String,

    /// The embedding of the chunk
    pub embedding: Vec<f32>,

    /// A summary of the chunk
    pub summary: String,

    /// A context string for the chunk
    pub context: String,

    /// Metadata for the chunk
    pub metadata: ChunkMetadata,
}

/// Metadata for a processed chunk
#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    /// The source URL of the chunk
    pub source_url: String,

    /// The position of the chunk in the original document
    pub position: usize,

    /// The heading of the chunk
    pub heading: Option<String>,
}

/// Process content from a crawled page
///
/// # Arguments
///
/// * `client` - The Gemini client
/// * `page` - The crawled page
/// * `config` - The processor configuration
///
/// # Returns
///
/// A vector of processed chunks
pub async fn process_content(
    client: &Client,
    page: CrawledPage,
    config: ProcessorConfig,
) -> Result<Vec<ProcessedChunk>, ProcessError> {
    debug!("Processing content from {}", page.url);

    // Chunk the markdown content
    let chunks = chunk_markdown(&page.content, &config.chunk_options)?;
    let mut processed_chunks = Vec::new();

    info!("Created {} chunks from {}", chunks.len(), page.url);

    // Process chunks in parallel with bounded concurrency
    let semaphore = Arc::new(Semaphore::new(5)); // Limit concurrent API calls

    let tasks = chunks
        .into_iter()
        .map(|chunk| {
            let permit = semaphore.clone().acquire_owned();
            let llm_model = config.llm_model.clone();
            let metadata = page.metadata.clone();
            let url = page.url.clone();
            let client = client.clone(); // Clone the client for each task

            tokio::spawn(async move {
                let _permit = permit.await?;
                // Create a new client with the same API key
                // wasn't seeing this contribute to a rate limit at all
                let embedding_client = Client::default_from_client(&client);

                // Generate embedding
                let embedding = generate_embedding(&embedding_client, &chunk.text).await?;

                // Generate summary using LLM
                let summary = generate_summary(&client, &chunk.text, &llm_model).await?;

                // Generate context string using LLM
                let context =
                    generate_context_string(&client, &chunk.text, &url, &metadata, &llm_model)
                        .await?;

                // Create chunk metadata
                let chunk_metadata = ChunkMetadata {
                    source_url: url,
                    position: chunk.position,
                    heading: chunk.heading,
                };

                // Create processed chunk
                let processed_chunk = ProcessedChunk {
                    text: chunk.text,
                    embedding,
                    summary,
                    context,
                    metadata: chunk_metadata,
                };

                Ok::<ProcessedChunk, ProcessError>(processed_chunk)
            })
        })
        .collect::<Vec<_>>();

    // Wait for all tasks to complete
    let results = future::join_all(tasks).await;

    // Process results
    for result in results {
        match result {
            Ok(Ok(chunk)) => processed_chunks.push(chunk),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(ProcessError::Task(format!("Task failed: {}", e))),
        }
    }

    Ok(processed_chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_options() {
        let options = ChunkOptions {
            target_chunk_size: 1000,
            overlap_size: 100,
        };

        assert_eq!(options.target_chunk_size, 1000);
        assert_eq!(options.overlap_size, 100);
    }

    #[test]
    fn test_processor_config() {
        let config = ProcessorConfig::builder()
            .target_chunk_size(1000)
            .overlap_size(100)
            .llm_model("test-model")
            .embedding_dimensions(768)
            .build();

        assert_eq!(config.chunk_options.target_chunk_size, 1000);
        assert_eq!(config.chunk_options.overlap_size, 100);
        assert_eq!(config.llm_model, "test-model");
        assert_eq!(config.embedding_dimensions, 768);
    }

    #[test]
    fn test_chunk_metadata() {
        let metadata = ChunkMetadata {
            source_url: "https://example.com".to_string(),
            position: 1,
            heading: Some("Test Heading".to_string()),
        };

        assert_eq!(metadata.source_url, "https://example.com");
        assert_eq!(metadata.position, 1);
        assert_eq!(metadata.heading.as_deref().unwrap(), "Test Heading");
    }

    #[test]
    fn test_processed_chunk() {
        let chunk = ProcessedChunk {
            text: "Test text".to_string(),
            embedding: vec![0.1, 0.2, 0.3],
            summary: "Test summary".to_string(),
            context: "Test context".to_string(),
            metadata: ChunkMetadata {
                source_url: "https://example.com".to_string(),
                position: 1,
                heading: Some("Test Heading".to_string()),
            },
        };

        assert_eq!(chunk.text, "Test text");
        assert_eq!(chunk.embedding, vec![0.1, 0.2, 0.3]);
        assert_eq!(chunk.summary, "Test summary");
        assert_eq!(chunk.context, "Test context");
        assert_eq!(chunk.metadata.source_url, "https://example.com");
        assert_eq!(chunk.metadata.position, 1);
        assert_eq!(chunk.metadata.heading.as_deref().unwrap(), "Test Heading");
    }
}
