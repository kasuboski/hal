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

mod client;
mod error;
mod models;
mod types;
mod chats;
mod files;
mod tunings;
mod caches;
mod batches;
mod http;

pub use client::Client;
pub use error::Error;

/// Re-export of types module for public use
pub mod prelude {
    pub use crate::types::*;
    pub use crate::error::Error;
    pub use crate::error::Result;
}