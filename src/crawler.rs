//! # Website Crawler Module for RAG
//!
//! This module provides comprehensive functionality for crawling websites,
//! extracting content, and preparing it for the RAG pipeline. It serves as the
//! first stage of the RAG workflow, responsible for gathering raw content.
//!
//! ## Key Components
//!
//! - `CrawlerConfig`: Configuration for the crawler, including depth, rate limits, etc.
//! - `CrawledPage`: Represents a processed web page with content and metadata
//! - `crawl_website`: Main function to crawl a website with the given configuration
//! - Content extraction utilities for converting HTML to clean, processable text
//!
//! ## Features
//!
//! - Configurable crawling depth and rate limits
//! - HTML to Markdown conversion for easier processing
//! - Metadata extraction (title, description, author, etc.)
//! - Respects robots.txt and can be configured for politeness
//! - Error handling for network and parsing issues
//!
//! ## Usage
//!
//! The crawler is typically the first step in a RAG pipeline, feeding content
//! to the processor module which then chunks it for embedding and indexing.

mod config;
mod content_extraction;
mod error;
mod spider_integration;
pub mod storage;

// Re-export important types and functions
pub use config::CrawlerConfig;
pub use content_extraction::extract_metadata;
pub use error::CrawlError;
pub use spider_integration::crawl_website;

use serde::{Deserialize, Serialize};

/// Represents a crawled page with its content and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawledPage {
    /// URL of the page
    pub url: String,

    /// Content of the page in Markdown format
    pub content: String,

    /// Metadata extracted from the page
    pub metadata: PageMetadata,
}

/// Metadata for a crawled page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMetadata {
    /// Title of the page
    pub title: Option<String>,

    /// Description of the page
    pub description: Option<String>,

    /// Publication date of the page
    pub publication_date: Option<chrono::DateTime<chrono::Utc>>,

    /// Author of the page
    pub author: Option<String>,

    /// Domain of the page
    pub domain: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_metadata() {
        let metadata = PageMetadata {
            title: Some("Test Page".to_string()),
            description: Some("Test description".to_string()),
            publication_date: None,
            author: Some("Test Author".to_string()),
            domain: "example.com".to_string(),
        };

        assert_eq!(metadata.title.as_deref().unwrap(), "Test Page");
        assert_eq!(metadata.description.as_deref().unwrap(), "Test description");
        assert_eq!(metadata.author.as_deref().unwrap(), "Test Author");
        assert_eq!(metadata.domain, "example.com");
    }
}
