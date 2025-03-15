//! Embedding generation functionality for the processor module

use crate::gemini::prelude::Content;
use crate::gemini::Client;
use crate::processor::error::ProcessError;
use tracing::{debug, trace};

/// Generate an embedding for a text
///
/// # Arguments
///
/// * `client` - The Gemini client to use
/// * `text` - The text to generate an embedding for
///
/// # Returns
///
/// A vector of floats representing the embedding
pub async fn generate_embedding(client: &Client, text: &str) -> Result<Vec<f32>, ProcessError> {
    debug!("Generating embedding for text of length {}", text.len());

    // Create a content object
    let content = Content::new().with_text(text);

    // Generate the embedding
    let response = client
        .models()
        .embed_content("text-embedding-004", content)
        .await
        .map_err(|e| {
            ProcessError::EmbeddingGeneration(format!("Failed to generate embedding: {}", e))
        })?;

    // Get the embedding values
    let values = response.embedding.values;

    trace!("Generated embedding with {} dimensions", values.len());
    Ok(values)
}
