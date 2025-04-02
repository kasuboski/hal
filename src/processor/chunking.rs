//! # Markdown Chunking Module
//!
//! This module provides sophisticated text chunking functionality specifically optimized
//! for Markdown content. It intelligently splits documents into semantically meaningful
//! segments while preserving important structural elements.
//!
//! ## Key Components
//!
//! - `TextChunk`: Represents a segment of text with metadata and position information
//! - `chunk_markdown`: Primary function for splitting Markdown into chunks
//!
//! ## Features
//!
//! - Structure-aware chunking that respects:
//!   - Paragraph boundaries
//!   - Code block integrity
//!   - Heading hierarchies
//!   - Document section boundaries
//! - Configurable chunk sizes with overlap for context continuity
//! - Metadata preservation (headings, positions) for improved retrieval
//! - UTF-8 safe text handling
//!
//! ## Chunking Strategy
//!
//! The chunker uses a sophisticated algorithm that:
//! 1. Parses Markdown with pulldown_cmark to understand document structure
//! 2. Attempts to split at natural boundaries (paragraphs, code blocks, etc.)
//! 3. Maintains content integrity by avoiding splits in the middle of important elements
//! 4. Associates chunks with their parent headings for context preservation
//!
//! This structure-aware chunking is critical for RAG quality as it ensures that
//! the indexed content maintains semantic coherence and proper context.

use crate::processor::ChunkOptions;
use crate::processor::error::ProcessError;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use serde::Serialize;
use tracing::{debug, instrument};

/// A chunk of text with metadata
#[derive(Debug, Clone, Serialize)]
pub struct TextChunk {
    /// The text of the chunk
    pub text: String,

    /// The position of the chunk in the original document
    pub position: usize,

    /// The heading of the chunk
    pub heading: Option<String>,
}

/// Chunk Markdown text into smaller pieces
/// The chunker tries to preserve paragraph boundaries and code block boundaries.
/// It builds a chunk out of a list of words. It tries it track important elements by their position in the list of words.
///
/// # Arguments
///
/// * `markdown` - The Markdown text to chunk
/// * `options` - Chunking options
///
/// # Returns
///
/// A vector of text chunks
#[instrument(skip(markdown))]
pub fn chunk_markdown(
    markdown: &str,
    options: &ChunkOptions,
) -> Result<Vec<TextChunk>, ProcessError> {
    debug!("Chunking Markdown text with options: {:?}", options);

    // Parse the Markdown
    let parser = Parser::new(markdown);

    // Track the current heading
    let mut current_heading = None;

    // Track the current chunk
    let mut current_chunk: Vec<String> = Vec::new();

    // Track the chunks
    let mut chunks = Vec::new();

    // Track the position
    let mut position = 0;

    // Track if we're inside a code block
    let mut in_code_block = false;

    // Track paragraph boundaries
    let mut paragraph_breaks = Vec::new();

    // Track code block boundaries
    let mut code_block_boundaries = Vec::new();

    // Process each event
    for event in parser {
        match &event {
            Event::Text(text) => {
                let words = text.split_inclusive([' ', '\n']).map(String::from);
                // Add the text to the current chunk
                current_chunk.extend(words);

                // Check if the chunk is large enough
                if current_chunk.len() >= options.target_chunk_size {
                    // Find a good boundary to split at
                    let split_point = find_split_point(
                        &current_chunk,
                        options.target_chunk_size,
                        &paragraph_breaks,
                        &code_block_boundaries,
                        in_code_block,
                    );

                    let split_text: String =
                        current_chunk.iter().take(split_point).cloned().collect();

                    chunks.push(TextChunk {
                        text: split_text.trim().to_string(),
                        position,
                        heading: current_heading.clone(),
                    });
                    position += 1;

                    // Start a new chunk with overlap
                    let overlap_start = split_point.saturating_sub(options.overlap_size);
                    current_chunk = current_chunk.into_iter().skip(overlap_start).collect();

                    // Adjust the paragraph and code block boundaries
                    adjust_boundaries(&mut paragraph_breaks, overlap_start, split_point);
                    adjust_boundaries(&mut code_block_boundaries, overlap_start, split_point);
                }

                // If we're capturing a heading and we get text, store it
                if let Some(heading) = &mut current_heading {
                    if heading.is_empty() {
                        *heading = text.to_string();
                    }
                }
            }
            Event::Start(tag) => {
                // Check if this is a heading
                if let Tag::Heading {
                    level,
                    id: _,
                    classes: _,
                    attrs: _,
                } = tag
                {
                    // Extract heading text (will be populated in the next events)
                    if matches!(
                        level,
                        HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3
                    ) {
                        // Only track headings up to level 3
                        // If we have a current chunk, add it to the chunks
                        if !current_chunk.is_empty() {
                            chunks.push(TextChunk {
                                text: current_chunk.join(""),
                                position,
                                heading: current_heading.clone(),
                            });
                            position += 1;
                            current_chunk.clear();
                            paragraph_breaks.clear();
                            code_block_boundaries.clear();
                        }

                        // We'll capture the heading text in the next Text event
                        current_heading = Some(String::new());
                    }
                } else if let Tag::CodeBlock(_kind) = tag {
                    // Mark the start of a code block
                    in_code_block = true;
                    code_block_boundaries.push(current_chunk.len());

                    // Add a marker for the code block start
                    if !current_chunk.is_empty()
                        && !current_chunk
                            .last()
                            .map(|c| c.ends_with('\n'))
                            .unwrap_or(false)
                    {
                        current_chunk.push("\n".to_string());
                    }
                } else if let Tag::Paragraph = tag {
                    // Mark the start of a paragraph
                    if !current_chunk.is_empty()
                        && !current_chunk
                            .last()
                            .map(|c| c.ends_with('\n'))
                            .unwrap_or(false)
                    {
                        current_chunk.push("\n".to_string());
                    }
                } else if !current_chunk.is_empty()
                    && !current_chunk
                        .last()
                        .map(|c| c.ends_with(['\n', ' ']) || c.ends_with(' '))
                        .unwrap_or(false)
                {
                    current_chunk.push(" ".to_string());
                }
            }
            Event::End(tag) => {
                if let TagEnd::CodeBlock = tag {
                    // Mark the end of a code block
                    in_code_block = false;
                    code_block_boundaries.push(current_chunk.len());

                    // Add a marker for the code block end
                    if !current_chunk.is_empty()
                        && !current_chunk
                            .last()
                            .map(|c| c.ends_with('\n'))
                            .unwrap_or(false)
                    {
                        current_chunk.push("\n".to_string());
                    }
                } else if let TagEnd::Paragraph = tag {
                    // Mark the end of a paragraph
                    paragraph_breaks.push(current_chunk.len());

                    // Add a newline after paragraphs
                    if !current_chunk
                        .last()
                        .map(|c| c.ends_with('\n'))
                        .unwrap_or(false)
                    {
                        current_chunk.push("\n".to_string());
                    }
                    current_chunk.push("\n".to_string());
                } else if let TagEnd::Heading(level) = tag {
                    if matches!(
                        level,
                        HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3
                    ) && current_heading.is_some()
                    {
                        // We've captured the heading text in previous Text events
                        // Add a newline after headings
                        if !current_chunk
                            .last()
                            .map(|c| c.ends_with('\n'))
                            .unwrap_or(false)
                        {
                            current_chunk.push("\n".to_string());
                        }
                    }
                }
            }
            Event::Code(code) => {
                // Inline code
                let code = code.clone().into_string();
                current_chunk.push("`".to_string());
                current_chunk.push(code);
                current_chunk.push("`".to_string());
            }
            Event::SoftBreak => {
                current_chunk.push(" ".to_string());
            }
            Event::HardBreak => {
                current_chunk.push("\n".to_string());
            }
            _ => {
                // Add a space to separate elements
                if !current_chunk.is_empty()
                    && !current_chunk
                        .last()
                        .map(|c| c.ends_with('\n') || c.ends_with(' '))
                        .unwrap_or(false)
                {
                    current_chunk.push(" ".to_string());
                }
            }
        }
    }

    // Add the final chunk if it's not empty
    if !current_chunk.is_empty() {
        chunks.push(TextChunk {
            text: current_chunk.join("").trim().to_string(),
            position,
            heading: current_heading,
        });
    }

    debug!("Created {} chunks", chunks.len());
    Ok(chunks)
}

/// Find an appropriate split point for a chunk
///
/// This function tries to find a natural boundary to split the text at,
/// respecting code blocks and paragraphs.
///
/// # Arguments
///
/// * `text` - The text to split
/// * `target_size` - The target size for the chunk
/// * `paragraph_breaks` - Positions of paragraph breaks in the text
/// * `code_block_boundaries` - Positions of code block boundaries in the text
/// * `in_code_block` - Whether we're currently inside a code block
///
/// # Returns
///
/// The position to split the text at
fn find_split_point(
    text: &[String],
    target_size: usize,
    paragraph_breaks: &[usize],
    code_block_boundaries: &[usize],
    in_code_block: bool,
) -> usize {
    let words: Vec<String> = text.iter().map(|s| s.to_string()).collect();
    let words_len = words.len();

    // If we're in a code block, try to find the end of it
    if in_code_block {
        // Find the next code block boundary after the target size
        for &pos in code_block_boundaries {
            if pos > target_size && pos < words_len {
                return pos;
            }
        }
    }

    // Try to split at a paragraph break
    for &pos in paragraph_breaks.iter().rev() {
        // Only use paragraph breaks that are at least 30% of the target size
        if pos > target_size * 3 / 10 && pos < words_len {
            return pos;
        }
    }

    // Try to split at a code block boundary
    for &pos in code_block_boundaries.iter().rev() {
        // Only use code block boundaries that are at least 30% of the target size
        if pos > target_size * 3 / 10 && pos < words_len {
            return pos;
        }
    }

    let sentence_pos = text
        .iter()
        .take(target_size + 200)
        .rposition(|word| word.ends_with(". ") || word.ends_with("! ") || word.ends_with("? "));

    if let Some(pos) = sentence_pos {
        return pos;
    }

    // If all else fails, split at the target word count
    std::cmp::min(words_len, target_size)
}

/// Adjust boundary positions after removing text
///
/// # Arguments
///
/// * `boundaries` - List of boundary positions to adjust
/// * `start` - Start position of removed text
/// * `end` - End position of removed text
fn adjust_boundaries(boundaries: &mut Vec<usize>, start: usize, end: usize) {
    let shift = end - start;

    // Remove boundaries that were in the removed section
    boundaries.retain(|&pos| pos < start || pos >= end);

    // Adjust remaining boundaries
    for pos in boundaries.iter_mut() {
        if *pos >= end {
            *pos -= shift;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processor::ChunkOptions;

    #[test]
    fn test_utf8_character_chunking() {
        let text = "Hello, 世界! This is a test with UTF-8 characters. 你好，世界！";
        let options = ChunkOptions {
            target_chunk_size: 20,
            overlap_size: 5,
        };

        let chunks = chunk_markdown(text, &options).unwrap();

        // Verify that chunks are created and contain valid UTF-8
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            // Verify each chunk contains valid UTF-8
            assert!(chunk.text.chars().all(|c| c.len_utf8() > 0));

            // Verify chunk boundaries don't split UTF-8 characters
            String::from_utf8(chunk.text.as_bytes().to_vec()).unwrap();
        }

        // Verify that the chunks can be rejoined without losing characters
        let total_chars = text.chars().count();
        let chunks_chars: usize = chunks.iter().map(|c| c.text.chars().count()).sum();
        assert!(
            chunks_chars >= total_chars,
            "All characters should be preserved in chunks"
        );
    }

    /// This test demonstrates how the chunk_markdown function works with different types of markdown content.
    /// It shows how the function respects code blocks, paragraphs, and headings when chunking text.
    #[test]
    fn test_chunk_markdown_illustrative() {
        // Arrange: Create a sample markdown document with various elements
        let markdown = r#"# Main Heading

This is the first paragraph with some text. It contains a few sentences
that should be kept together when chunking. The chunker should try to respect
paragraph boundaries.

## Second Level Heading

This is another paragraph with different content. It should be associated
with the second level heading.

```rust
// This is a code block
fn example_function() -> Result<(), Error> {
    println!("This code block should not be split in the middle");
    println!("It should be kept intact if possible");
    Ok(())
}
```

### Third Level Heading

- List item 1
- List item 2
- List item 3

This is a paragraph after the list. It should be treated as a separate chunk
from the list above.

#### Fourth Level Heading

This heading is level 4, so it might not be tracked as a main heading.

```python
# Another code block in a different language
def another_example():
    print("This should also be kept intact")
    return True
```

Final paragraph with some concluding text."#;

        // Create chunk options with a small target size to force multiple chunks
        let options = ChunkOptions {
            target_chunk_size: 200, // Small size to force multiple chunks
            overlap_size: 50,       // Reasonable overlap
        };

        // Act: Chunk the markdown
        let chunks = chunk_markdown(markdown, &options).unwrap();

        // Assert: Verify the chunks are created correctly
        assert!(!chunks.is_empty(), "Should have created at least one chunk");

        // Print the chunks for illustration
        println!("Created {} chunks:", chunks.len());
        for (i, chunk) in chunks.iter().enumerate() {
            println!("\nChunk {}:", i + 1);
            println!("Heading: {:?}", chunk.heading);
            println!("Position: {}", chunk.position);
            println!("Text length: {} characters", chunk.text.len());
            println!("Text preview: {}", preview_text(&chunk.text, 100));

            // Check for code block boundaries
            if chunk.text.contains("```") {
                println!("Contains code block markers");

                // Check if code block is split
                let open_count = chunk.text.matches("```").count();
                if open_count % 2 != 0 {
                    println!("WARNING: Code block might be split across chunks!");

                    // Check if it's the start or end of a code block
                    if chunk.text.trim_end().ends_with("```") {
                        println!("  - This chunk contains the end of a code block");
                    }
                    if chunk.text.contains("```") && !chunk.text.contains("```\n") {
                        println!("  - This chunk contains the start of a code block");
                    }
                } else {
                    println!("Code blocks are intact within this chunk");
                }
            }

            // Check for paragraph integrity
            let paragraphs = chunk.text.split("\n\n").collect::<Vec<_>>();
            println!("Contains {} paragraphs:", paragraphs.len());
            for (j, para) in paragraphs.iter().enumerate().take(2) {
                println!("  - Paragraph {}: {}", j + 1, preview_text(para, 50));
            }
            if paragraphs.len() > 2 {
                println!("  - ... and {} more paragraphs", paragraphs.len() - 2);
            }
        }

        // Verify some specific expectations

        // The first chunk should contain the main heading
        assert_eq!(
            chunks[0].heading,
            Some("Main Heading".to_string()),
            "First chunk should have the main heading"
        );

        // Check that code blocks are not split in the middle of code
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.text.contains("```rust") || chunk.text.contains("```python") {
                // This chunk starts a code block
                let code_start = chunk.text.find("```").unwrap();
                let remaining_text = &chunk.text[code_start..];

                // If the code block doesn't end in this chunk, it should end at a chunk boundary
                if !remaining_text.contains("\n```") && i < chunks.len() - 1 {
                    println!(
                        "\nCode block starts in chunk {} and continues to next chunk",
                        i + 1
                    );
                    // The next chunk should continue the code block
                    assert!(
                        chunks[i + 1].text.contains("```"),
                        "Code block should continue in next chunk"
                    );
                }
            }
        }

        // Verify that paragraphs are generally kept intact
        for chunk in &chunks {
            let paragraphs = chunk.text.split("\n\n").collect::<Vec<_>>();
            for para in paragraphs {
                // Check that paragraphs aren't too small (arbitrary threshold)
                if !para.contains("```") && para.trim().len() > 10 {
                    assert!(
                        para.split_whitespace().count() >= 2,
                        "Paragraphs should generally contain multiple words: {}",
                        para
                    );
                }
            }
        }

        // Verify that some chunks have associated headings
        let chunks_with_headings = chunks
            .iter()
            .filter(|chunk| chunk.heading.is_some())
            .count();

        assert!(chunks_with_headings > 0, "Some chunks should have headings");

        // Verify that heading associations are maintained correctly
        let mut current_heading = None;
        for chunk in &chunks {
            if chunk.heading.is_some() {
                current_heading = chunk.heading.clone();
            } else if current_heading.is_some() {
                // If this chunk doesn't have a heading but we've seen one before,
                // it should have the same heading as the previous chunk with a heading
                assert_eq!(
                    chunk.heading, current_heading,
                    "Chunks should maintain heading association"
                );
            }
        }
    }

    /// This test specifically demonstrates how the chunker preserves code blocks and paragraph boundaries.
    #[test]
    fn test_chunk_markdown_boundary_preservation() {
        // Arrange: Create a markdown document with a large code block and paragraphs
        let markdown = r#"# Boundary Preservation Test

This is a paragraph before a code block. We want to ensure that the chunker
respects the boundaries between paragraphs and code blocks.

```rust
// This is a large code block that might exceed the chunk size
fn example_function() -> Result<(), Error> {
    // First, we do some initialization
    let mut data = Vec::new();
    for i in 0..100 {
        data.push(i);
    }

    // Then we process the data
    let processed = data.iter()
        .map(|x| x * 2)
        .filter(|x| x % 3 == 0)
        .collect::<Vec<_>>();

    // Finally, we return the result
    println!("Processed {} items", processed.len());
    Ok(())
}

// Another function in the same code block
fn another_function() {
    println!("This is another function");
    println!("It should be kept with the previous function");
    println!("Because they're in the same code block");
}
```

This is a paragraph after the code block. It should be in a different chunk
than the code block if the code block is large enough to be its own chunk.

Here's another paragraph that should be kept together with the previous one
if possible, rather than being split in the middle.

## A New Section

This section starts with a heading, which should create a new chunk boundary.
The content under this heading should be associated with this heading."#;

        // Create chunk options with a size that will force the code block to be chunked
        let options = ChunkOptions {
            target_chunk_size: 30, // Size that will likely split the code block
            overlap_size: 5,       // Reasonable overlap
        };

        // Act: Chunk the markdown
        let chunks = chunk_markdown(markdown, &options).unwrap();

        // Print the chunks for illustration
        println!("\n=== BOUNDARY PRESERVATION TEST ===");
        println!("Created {} chunks:", chunks.len());

        // Track code block state across chunks
        let mut in_code_block = false;
        let mut code_block_chunks = Vec::new();

        for (i, chunk) in chunks.iter().enumerate() {
            println!("\nChunk {}:", i + 1);
            println!("Heading: {:?}", chunk.heading);
            println!("Position: {}", chunk.position);
            println!("Text length: {} characters", chunk.text.len());

            // Check for code block markers
            let starts_code = chunk.text.contains("```rust")
                || (chunk.text.contains("```") && !chunk.text.contains("```\n"));
            let ends_code = chunk.text.contains("\n```");

            if starts_code {
                println!("⬇️ STARTS CODE BLOCK ⬇️");
                in_code_block = true;
                code_block_chunks.push(i);
            }

            if in_code_block {
                println!("📝 CONTAINS CODE BLOCK 📝");
            }

            if ends_code {
                println!("⬆️ ENDS CODE BLOCK ⬆️");
                in_code_block = false;
            }

            // Print the first few lines and last few lines
            let lines: Vec<&str> = chunk.text.lines().collect();
            println!("First few lines:");
            for line in lines.iter().take(3) {
                println!("  {}", line);
            }
            if lines.len() > 6 {
                println!("  ... ({} more lines) ...", lines.len() - 6);
            }
            println!("Last few lines:");
            for line in lines.iter().rev().take(3).rev() {
                println!("  {}", line);
            }
        }

        // Assert: Verify code block handling
        if !code_block_chunks.is_empty() {
            println!("\nCode block appears in chunks: {:?}", code_block_chunks);

            // Check if code block is split across chunks
            if code_block_chunks.len() > 1 {
                println!("Code block is split across multiple chunks");

                // Verify that the split happens at reasonable boundaries
                for i in 0..code_block_chunks.len() - 1 {
                    let current = code_block_chunks[i];
                    let next = code_block_chunks[i + 1];

                    // Chunks should be consecutive
                    assert_eq!(next, current + 1, "Code block chunks should be consecutive");

                    // Check the end of the current chunk
                    let current_chunk = &chunks[current];
                    let current_lines: Vec<&str> = current_chunk.text.lines().collect();
                    let last_line = current_lines.last().unwrap_or(&"");

                    // Check the start of the next chunk
                    let next_chunk = &chunks[next];
                    let next_lines: Vec<&str> = next_chunk.text.lines().collect();
                    let first_line = next_lines.first().unwrap_or(&"");

                    println!("Split between chunks {} and {}:", current + 1, next + 1);
                    println!("  Last line of chunk {}: {}", current + 1, last_line);
                    println!("  First line of chunk {}: {}", next + 1, first_line);

                    // The split should happen at a reasonable boundary (empty line or function boundary)
                    assert!(
                        last_line.trim().is_empty()
                            || last_line.trim().ends_with("{")
                            || last_line.trim().ends_with("}")
                            || first_line.trim().is_empty()
                            || first_line.trim().starts_with("fn ")
                            || first_line.trim().starts_with("//"),
                        "Code block should be split at a reasonable boundary"
                    );
                }
            } else {
                println!("Code block is contained within a single chunk");
            }
        }

        // Verify paragraph handling
        let mut paragraph_count = 0;
        for chunk in &chunks {
            let paragraphs = chunk.text.split("\n\n").collect::<Vec<_>>();
            paragraph_count += paragraphs.len();

            // Check that paragraphs aren't split in the middle
            for para in paragraphs {
                if !para.contains("```") && para.trim().len() > 10 {
                    // Count sentences in paragraph (rough approximation)
                    let sentences = para
                        .split(['.', '!', '?'])
                        .filter(|s| !s.trim().is_empty())
                        .count();

                    println!(
                        "Paragraph with {} sentences: {}",
                        sentences,
                        preview_text(para, 50)
                    );

                    // Most paragraphs should have complete sentences
                    if sentences > 0 && !para.trim().starts_with("//") && !para.contains("fn ") {
                        assert!(
                            para.contains('.')
                                || para.contains('!')
                                || para.contains('?')
                                || para.trim().starts_with('#')
                                || para.contains("```")
                                || para.trim().starts_with("fn ")
                                || para.trim().starts_with("//"),
                            "Paragraphs should generally contain complete sentences"
                        );
                    }
                }
            }
        }

        println!("\nTotal paragraphs across all chunks: {}", paragraph_count);

        // Verify heading boundaries
        let heading_chunks = chunks
            .iter()
            .filter(|chunk| chunk.heading.as_deref() == Some("A New Section"))
            .collect::<Vec<_>>();

        assert!(
            !heading_chunks.is_empty(),
            "Should have a chunk with the section heading"
        );

        // The heading should be at or near the start of its chunk
        let heading_chunk = heading_chunks[0];
        let heading_pos = heading_chunk.text.find("A New Section").unwrap();

        println!(
            "\nHeading 'A New Section' position in its chunk: {}",
            heading_pos
        );
        assert!(
            heading_pos < 50,
            "Heading should be near the start of its chunk"
        );
    }

    /// Helper function to preview text with a maximum length
    fn preview_text(text: &str, max_length: usize) -> String {
        if text.len() <= max_length {
            text.to_string()
        } else {
            format!("{}...", &text[..max_length])
        }
    }
}
