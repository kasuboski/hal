//! # LLM Integration Module
//!
//! This module provides integration with Language Models for enhancing the RAG pipeline
//! through AI-generated summaries and context information. It leverages LLMs to improve
//! the semantic understanding and retrieval quality of processed content.
//!
//! ## Key Components
//!
//! - `generate_summary`: Creates concise summaries of document content
//! - `generate_context_string`: Produces rich context information for document chunks
//!
//! ## Features
//!
//! - LLM-powered document summarization for high-level understanding
//! - Context generation that combines metadata and content for improved relevance
//! - Integration with the agent framework for conversation-like interactions
//! - Structured prompting for consistent, high-quality outputs
//! - Instrumentation with tracing for monitoring and performance analysis
//!
//! This module represents a key differentiator for advanced RAG systems by using
//! AI to enhance the semantic quality of chunked content. The generated summaries
//! and context strings help bridge the gap between raw text and the nuanced understanding
//! needed for effective retrieval augmentation.

use crate::model::Client;
use crate::processor::error::ProcessError;
use rig::agent::AgentBuilder;
use rig::completion::{CompletionModel, Prompt};
use rig::embeddings::EmbeddingModel;
use tracing::{debug, instrument, trace};

/// Generate a summary for a text
///
/// # Arguments
///
/// * `client` - The client to use
/// * `text` - The text to generate a summary for
/// * `model` - The LLM model to use
///
/// # Returns
///
/// A summary of the text
#[instrument(skip(client, text))]
pub async fn generate_summary<C, E>(
    client: &Client<C, E>,
    text: &str,
    _model: &str,
) -> Result<String, ProcessError>
where
    C: CompletionModel,
    E: EmbeddingModel,
{
    debug!("Generating summary for text of length {}", text.len());

    let completion = client.completion().clone();
    let agent = AgentBuilder::new(completion)
        .preamble("Summarize the following text in a concise paragraph:\n")
        .build();

    let summary = agent
        .prompt(text)
        .await
        .map_err(|e| ProcessError::Llm(format!("Failed to generate summary: {}", e)))?;

    trace!("Generated summary of length {}", summary.len());
    Ok(summary)
}

/// Generate a context string for a text
///
/// # Arguments
///
/// * `client` - The client to use
/// * `text` - The text to generate a context string for
/// * `url` - The URL of the source
/// * `metadata` - Metadata about the source
/// * `model` - The LLM model to use
///
/// # Returns
///
/// A context string for the text
#[instrument(skip(client, summary))]
pub async fn generate_context_string<C, E>(
    client: &Client<C, E>,
    text: &str,
    url: &str,
    summary: &str,
    metadata: &crate::crawler::PageMetadata,
    _model: &str,
) -> Result<String, ProcessError>
where
    C: CompletionModel,
    E: EmbeddingModel,
{
    debug!(
        "Generating context string for text of length {}",
        text.len()
    );

    let prompt=    format!(
            "Generate a concise context string for the following text. The context string should help a user understand where this information comes from and its relevance.\n\n\
            Source URL: {}\n\
            Title: {}\n\
            Description: {}\n\
            Page Summary: {}\n\
            Domain: {}\n\n\
            Text:\n",
            url,
            metadata.title.as_deref().unwrap_or("Unknown"),
            metadata.description.as_deref().unwrap_or("No description available"),
            summary,
            metadata.domain
        );
    let completion = client.completion().clone();
    let context = AgentBuilder::new(completion)
        .build()
        .prompt(prompt)
        .await
        .map_err(|e| ProcessError::Llm(format!("Failed to generate context string: {}", e)))?;

    trace!("Generated context string of length {}", context.len());
    Ok(context)
}
