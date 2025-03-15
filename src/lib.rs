//! # HAL - Google Gemini AI API Client for Rust
//!
//! This crate provides an idiomatic Rust interface to Google's Generative AI APIs,
//! specifically the Gemini models. It supports both the Gemini Developer API (via API key)
//! and Vertex AI integration.
//!
//! ## Features
//!
//! - Client configuration for both API key and Vertex AI authentication
//! - Content generation (text, images, etc.)
//! - Chat sessions for multi-turn conversations
//! - Embedding generation
//! - File handling
//! - Tuning operations
//! - Async API with Tokio
//! - Website crawling and indexing for RAG (Retrieval-Augmented Generation)
//!
//! ## Example
//!
//! ```rust,no_run
//! use hal::Client;
//! use hal::types::Content;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a client with API key
//!     let client = Client::with_api_key("your-api-key");
//!
//!     // Generate content
//!     let response = client.models().generate_content(
//!         "gemini-1.5-pro",
//!         Content::new().with_text("Tell me a story about a robot learning to feel emotions.")
//!     ).await?;
//!
//!     println!("{}", response.text());
//!     Ok(())
//! }
//! ```

mod error;
mod gemini;
mod markdown;
// RAG feature modules
pub mod crawler;
pub mod index;
pub mod processor;
pub mod search;

pub use error::Error;
pub use gemini::Client;
pub use markdown::format_markdown;

/// Re-export of types module for public use
pub mod prelude {
    pub use crate::gemini::prelude::*;
}
