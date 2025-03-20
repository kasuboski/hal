//! # HAL - Retrieval Augmented Generation Framework for Rust
//!
//! HAL is a comprehensive Retrieval-Augmented Generation (RAG) framework for Rust,
//! providing the tools needed to build AI-powered applications with semantic search
//! capabilities. It integrates web crawling, content processing, vector storage,
//! and LLM integration into a cohesive system.
//!
//! ## Core Components
//!
//! - **LLM Client**: Rate-limited clients for different model providers
//! - **Web Crawler**: Built on the `spider` library for efficient content extraction
//! - **Content Processor**: Smart text chunking and processing for RAG applications
//! - **Vector Database**: LibSQL-based storage for embeddings and content
//! - **Semantic Search**: Vector-based search with RAG integration
//!
//! ## Features
//!
//! - Flexible client configuration for different LLM providers
//! - Rate-limited content generation with configurable quotas
//! - Efficient embedding generation for semantic search
//! - Comprehensive RAG pipeline:
//!   - Website crawling with depth and rate controls
//!   - Smart text chunking with configurable parameters
//!   - Vector indexing with metadata storage
//!   - Semantic search with relevance scoring
//! - Terminal-based chat interface
//! - Async API powered by Tokio
//! - Robust error handling and telemetry integration
//!
//! ## Example
//!
//! ```rust,no_run
//! use hal::model::Client;
//! use rig::{providers::gemini, agent::AgentBuilder};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a client with rate limiting
//!     let gemini_client = gemini::Client::new("your-api-key");
//!     let client = Client::new_gemini(gemini_client);
//!
//!     // Create an agent with a preamble
//!     let completion = client.completion().clone();
//!     let agent = AgentBuilder::new(completion)
//!         .preamble("You are a helpful assistant.")
//!         .build();
//!
//!     // Generate content using the agent
//!     let response = agent
//!         .prompt("Tell me about retrieval augmented generation.")
//!         .await?;
//!
//!     println!("{}", response);
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
