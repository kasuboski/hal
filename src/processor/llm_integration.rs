//! LLM integration functionality for the processor module

use crate::gemini::prelude::Content;
use crate::gemini::Client;
use crate::processor::error::ProcessError;
use tracing::{debug, trace};

/// Generate a summary for a text
///
/// # Arguments
///
/// * `client` - The Gemini client to use
/// * `text` - The text to generate a summary for
/// * `model` - The LLM model to use
///
/// # Returns
///
/// A summary of the text
pub async fn generate_summary(
    client: &Client,
    text: &str,
    model: &str,
) -> Result<String, ProcessError> {
    debug!("Generating summary for text of length {}", text.len());

    // Create a prompt
    let prompt = format!(
        "Summarize the following text in a concise paragraph:\n\n{}",
        text
    );

    // Create a content object
    let content = Content::new().with_text(prompt);

    // Generate the summary
    let response = client
        .models()
        .generate_content(model, None, vec![content])
        .await
        .map_err(|e| ProcessError::Llm(format!("Failed to generate summary: {}", e)))?;

    // Get the summary
    let summary = response.text();

    trace!("Generated summary of length {}", summary.len());
    Ok(summary)
}

/// Generate a context string for a text
///
/// # Arguments
///
/// * `client` - The Gemini client to use
/// * `text` - The text to generate a context string for
/// * `url` - The URL of the source
/// * `metadata` - Metadata about the source
/// * `model` - The LLM model to use
///
/// # Returns
///
/// A context string for the text
pub async fn generate_context_string(
    client: &Client,
    text: &str,
    url: &str,
    metadata: &crate::crawler::PageMetadata,
    model: &str,
) -> Result<String, ProcessError> {
    debug!(
        "Generating context string for text of length {}",
        text.len()
    );

    // Create a prompt
    let prompt = format!(
        "Generate a concise context string for the following text. The context string should help a user understand where this information comes from and its relevance.\n\n\
        Source URL: {}\n\
        Title: {}\n\
        Description: {}\n\
        Domain: {}\n\n\
        Text:\n{}",
        url,
        metadata.title.as_deref().unwrap_or("Unknown"),
        metadata.description.as_deref().unwrap_or("No description available"),
        metadata.domain,
        text
    );

    // Create a content object
    let content = Content::new().with_text(prompt);

    // Generate the context string
    let response = client
        .models()
        .generate_content(model, None, vec![content])
        .await
        .map_err(|e| ProcessError::Llm(format!("Failed to generate context string: {}", e)))?;

    // Get the context string
    let context = response.text();

    trace!("Generated context string of length {}", context.len());
    Ok(context)
}
