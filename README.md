# HAL - LLM Stuff

> ⚠️ **Warning**: This project is in very early development and is not ready for production use. APIs may change significantly.

HAL is maybe going to be an LLM chat client. It's mostly written by Claude.

## Features

- Content generation (text, images, etc.)
- Chat sessions for multi-turn conversations
- Embedding generation
- File handling
- Tuning operations
- Async API with Tokio

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
hal = "0.1.0"
```

## Quick Start

```rust
use hal::Client;
use hal::types::Content;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client with API key
    let client = Client::with_api_key("your-api-key");

    // Generate content
    let response = client.models().generate_content(
        "gemini-1.5-pro",
        Content::new().with_text("Tell me a story about a robot learning to feel emotions.")
    ).await?;

    println!("{}", response.text());
    Ok(())
}
```

## Examples

To try out the interactive chat example:

```bash
cargo run --example chat
```

## Development Status

This project is in its early stages and under active development. Many features are experimental and the API is subject to change. Please do not use it in production environments yet.
