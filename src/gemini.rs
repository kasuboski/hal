//! Gemini API implementation
//!
//! This module provides the core implementation for interacting with Google's Gemini API.

mod batches;
mod caches;
mod chats;
mod client;
mod files;
mod http;
mod models;
mod types;

pub use client::Client;

/// Re-export of types module for public use
pub mod prelude {
    pub use super::types::*;
    pub use crate::error::Error;
    pub use crate::error::Result;
}
