//! Markdown chunking functionality for the processor module

use crate::processor::error::ProcessError;
use crate::processor::ChunkOptions;
use pulldown_cmark::{Event, Parser};
use tracing::{debug, trace};

/// A chunk of text with metadata
#[derive(Debug, Clone)]
pub struct TextChunk {
    /// The text of the chunk
    pub text: String,

    /// The position of the chunk in the original document
    pub position: usize,

    /// The heading of the chunk
    pub heading: Option<String>,
}

/// Chunk Markdown text into smaller pieces
///
/// # Arguments
///
/// * `markdown` - The Markdown text to chunk
/// * `options` - Chunking options
///
/// # Returns
///
/// A vector of text chunks
pub fn chunk_markdown(
    markdown: &str,
    options: &ChunkOptions,
) -> Result<Vec<TextChunk>, ProcessError> {
    debug!("Chunking Markdown text with options: {:?}", options);

    // Parse the Markdown
    let parser = Parser::new(markdown);

    // Track the current heading
    let current_heading = None;

    // Track the current chunk
    let mut current_chunk = String::new();

    // Track the chunks
    let mut chunks = Vec::new();

    // Track the position
    let mut position = 0;

    // Process each event
    for event in parser {
        match event {
            Event::Text(text) => {
                // Add the text to the current chunk
                current_chunk.push_str(&text);

                // Check if the chunk is large enough
                if current_chunk.len() >= options.target_chunk_size {
                    // Add the chunk to the chunks
                    chunks.push(TextChunk {
                        text: current_chunk.clone(),
                        position,
                        heading: current_heading.clone(),
                    });
                    position += 1;

                    // Start a new chunk with overlap
                    let overlap_start = current_chunk.len().saturating_sub(options.overlap_size);
                    current_chunk = current_chunk[overlap_start..].to_string();
                }
            }
            Event::Start(tag) => {
                // Check if this is a heading
                if let Some(level) = get_heading_level(&tag) {
                    trace!("Found heading level {}", level);

                    // If we have a current chunk, add it to the chunks
                    if !current_chunk.trim().is_empty() {
                        chunks.push(TextChunk {
                            text: current_chunk.clone(),
                            position,
                            heading: current_heading.clone(),
                        });
                        position += 1;
                        current_chunk.clear();
                    }
                } else {
                    // Add a space to separate elements
                    if !current_chunk.is_empty() && !current_chunk.ends_with(' ') {
                        current_chunk.push(' ');
                    }
                }
            }
            Event::End(_) => {
                // Do nothing
            }
            _ => {
                // Add a space to separate elements
                if !current_chunk.is_empty() && !current_chunk.ends_with(' ') {
                    current_chunk.push(' ');
                }
            }
        }
    }

    // Add the final chunk if it's not empty
    if !current_chunk.trim().is_empty() {
        chunks.push(TextChunk {
            text: current_chunk,
            position,
            heading: current_heading,
        });
    }

    debug!("Created {} chunks", chunks.len());
    Ok(chunks)
}

/// Helper function to get the heading level from a tag
fn get_heading_level(tag: &pulldown_cmark::Tag) -> Option<usize> {
    // Convert the tag to a string and check if it starts with "Heading"
    let tag_str = format!("{:?}", tag);
    if tag_str.starts_with("Heading") {
        // Extract the level from the string
        if let Some(level_str) = tag_str.chars().nth(7) {
            if let Some(level) = level_str.to_digit(10) {
                return Some(level as usize);
            }
        }
    }
    None
}
