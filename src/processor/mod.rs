//! Content processor module for RAG
//!
//! This module provides functionality for processing content,
//! including chunking, embedding generation, and LLM integration.

mod chunking;
mod config;
mod error;
mod llm_integration;

pub use chunking::chunk_markdown;
pub use config::{ChunkOptions, ProcessorConfig};
pub use error::ProcessError;
pub use llm_integration::{generate_context_string, generate_summary};

use crate::crawler::CrawledPage;
use crate::model::Client;
use futures::future;
use rig::{
    completion::CompletionModel,
    embeddings::{Embedding, EmbeddingModel},
};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, info, instrument};

/// Represents a processed chunk with its embedding and context
#[derive(Debug, Clone)]
pub struct ProcessedChunk {
    /// The text of the chunk
    pub text: String,

    /// The embedding of the chunk
    pub embedding: Embedding,

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

/// Generate an embedding from combined text and context
///
/// # Arguments
///
/// * `client` - The client to use
/// * `text` - The text content
/// * `context` - The context information
///
/// # Returns
///
/// A vector of floats representing the embedding
#[instrument(skip(client))]
pub async fn generate_combined_embedding<C, E>(
    client: &Client<C, E>,
    text: &str,
    context: &str,
) -> Result<Embedding, ProcessError>
where
    C: CompletionModel,
    E: EmbeddingModel,
{
    // Combine the text and context
    let combined_text = format!("Context: {}\nText: {}", text, context);

    // Generate embedding using the embedding model
    let embeddings = client
        .embedding()
        .embed_texts(vec![combined_text])
        .await?
        .first()
        .ok_or(ProcessError::EmbeddingProcessing(
            "failed to extract embedding".to_string(),
        ))?
        .clone();
    Ok(embeddings)
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
#[instrument(skip(client, page), fields(url = page.url))]
pub async fn process_content<C, E>(
    client: &Client<C, E>,
    page: CrawledPage,
    config: ProcessorConfig,
) -> Result<Vec<ProcessedChunk>, ProcessError>
where
    C: CompletionModel + Clone + Send + Sync + 'static,
    E: EmbeddingModel + Clone + Send + Sync + 'static,
{
    debug!("Processing content from {}", page.url);

    // Chunk the markdown content
    let chunks = chunk_markdown(&page.content, &config.chunk_options)?;
    let mut processed_chunks = Vec::new();

    // Generate summary of page to use for context
    let summary = generate_summary(&client, &page.content, &config.llm_model).await?;

    info!("Created {} chunks from {}", chunks.len(), page.url);

    // Process chunks in parallel with bounded concurrency
    let semaphore = Arc::new(Semaphore::new(5)); // Limit concurrent API calls

    let tasks = chunks
        .into_iter()
        .filter_map(|chunk| {
            let permit = semaphore.clone().acquire_owned();
            let llm_model = config.llm_model.clone();
            let metadata = page.metadata.clone();
            let url = page.url.clone();
            let summary = summary.clone();
            let client = client.clone();

            if chunk.text.len() <= 100 {
                debug!("Skipping small chunk");
                return None;
            }
            Some(tokio::spawn(async move {
                let _permit = permit
                    .await
                    .map_err(|e| ProcessError::Semaphore(e.to_string()));

                // Generate context string using LLM
                let context = generate_context_string(
                    &client,
                    &chunk.text,
                    &url,
                    &summary,
                    &metadata,
                    &llm_model,
                )
                .await?;

                // Generate embedding from combined text and context
                let embedding = generate_combined_embedding(&client, &chunk.text, &context).await?;

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
                    context,
                    metadata: chunk_metadata,
                };

                Ok::<ProcessedChunk, ProcessError>(processed_chunk)
            }))
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
        let embedding = Embedding {
            document: "Test text".to_string(),
            vec: vec![0.1, 0.2, 0.3],
        };
        let chunk = ProcessedChunk {
            text: "Test text".to_string(),
            embedding,
            context: "Test context".to_string(),
            metadata: ChunkMetadata {
                source_url: "https://example.com".to_string(),
                position: 1,
                heading: Some("Test Heading".to_string()),
            },
        };

        assert_eq!(chunk.text, "Test text");
        assert_eq!(chunk.embedding.vec, vec![0.1, 0.2, 0.3]);
        assert_eq!(chunk.context, "Test context");
        assert_eq!(chunk.metadata.source_url, "https://example.com");
        assert_eq!(chunk.metadata.position, 1);
        assert_eq!(chunk.metadata.heading.as_deref().unwrap(), "Test Heading");
    }
}
