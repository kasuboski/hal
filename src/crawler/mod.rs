//! Website crawler module for RAG
//!
//! This module provides functionality for crawling websites,
//! extracting content, and converting HTML to Markdown.

mod config;
mod content_extraction;
mod error;
mod spider_integration;

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
