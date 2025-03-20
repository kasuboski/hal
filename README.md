# HAL - LLM-powered RAG Framework

> ⚠️ **Warning**: This project is in very early development and is not ready for production use. APIs may change significantly.

HAL is a Retrieval-Augmented Generation (RAG) framework for Rust, providing tools for working with large language models. It features a robust web crawler, content processor, vector database, and semantic search capabilities.

## Features

- LLM client for text generation with rate limiting
- Embedding generation and vector search
- TUI-based chat interface
- Web crawler for content extraction
- Markdown processing with smart chunking
- Vector indexing with LibSQL
- Semantic search with RAG integration
- Async API with Tokio

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
hal = "0.1.0"
```

## Quick Start

```rust
use hal::model::Client;
use rig::{providers::gemini, agent::AgentBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client with rate limiting
    let gemini_client = gemini::Client::new("your-api-key");
    let client = Client::new_gemini(gemini_client);

    // Create an agent with a preamble
    let completion = client.completion().clone();
    let agent = AgentBuilder::new(completion)
        .preamble("You are a helpful assistant.")
        .build();

    // Generate content using the agent
    let response = agent
        .prompt("Tell me about retrieval augmented generation.")
        .await?;

    println!("{}", response);
    Ok(())
}
```

## CLI Commands

HAL provides a command-line interface with several useful commands:

```bash
# Start an interactive chat session
cargo run -- chat

# Crawl a website for content
cargo run -- crawl https://example.com --depth 2

# Index crawled content for RAG
cargo run -- index https://example.com --chunk-size 500

# Search the indexed content
cargo run -- search "your query here"

# List indexed websites
cargo run -- list --details
```

## Development Status

This project is under active development. The API may change significantly between versions. While it's functional for personal and experimental use, it is not yet recommended for production environments.
