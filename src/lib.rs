//! # HAL - Language Model Client with RAG for Rust
//!
//! This crate provides an idiomatic Rust interface for working with large language models,
//! featuring a robust Retrieval-Augmented Generation (RAG) framework. It supports various
//! model providers and includes comprehensive tools for building AI-powered applications.
//!
//! ## Features
//!
//! - Flexible client configuration for different model providers
//! - Rate-limited content generation with automatic retries
//! - Chat sessions for multi-turn conversations
//! - Efficient embedding generation with caching
//! - Comprehensive RAG framework:
//!   - Website crawling and content extraction
//!   - Smart text chunking and processing
//!   - Vector indexing with LibSQL
//!   - Semantic search capabilities
//! - Async API with Tokio
//! - Robust error handling and logging
//!
//! ## Example
//!
//! ```rust,no_run
//! use hal::model::Client;
//! use rig::{providers::gemini, agent::AgentBuilder};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a Gemini client with API key and rate limiting
//!     let gemini_client = gemini::Client::new("your-api-key");
//!     let client = Client::new_gemini(gemini_client);
//!
//!     // Create an agent with a preamble for storytelling
//!     let completion = client.completion().clone();
//!     let agent = AgentBuilder::new(completion)
//!         .preamble("You are a creative storyteller. Tell an engaging story based on the given prompt:")
//!         .build();
//!
//!     // Generate a story using the agent
//!     let story = agent
//!         .prompt("Tell me a story about a robot learning to feel emotions.")
//!         .await?;
//!
//!     println!("{}", story);
//!     Ok(())
//! }
//! ```

mod error;
mod markdown;
pub mod model;

// RAG feature modules
pub mod crawler;
pub mod index;
pub mod processor;
pub mod search;

pub use error::Error;
pub use markdown::format_markdown;

/// Re-export of types module for public use
pub mod prelude {
    pub use crate::error::Error;
    pub use crate::error::Result;
}
