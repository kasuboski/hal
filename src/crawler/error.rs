//! # Crawler Error Types Module
//!
//! This module defines error types specific to the crawler component of the RAG pipeline.
//! It provides structured error handling for various failure modes during web crawling.
//!
//! ## Key Components
//!
//! - `CrawlError`: Enum representing different types of crawler failures
//! - Conversion implementations to integrate with the crate's global error system
//!
//! ## Features
//!
//! - Specialized error types for different crawling failure scenarios
//! - HTTP errors for network and request issues
//! - Parsing errors for HTML and URL handling
//! - Rate limiting and robots.txt compliance errors
//! - Integration with the crate's main error type for consistent error handling
//!
//! These error types enable proper debugging and user feedback during the
//! content acquisition phase of the RAG pipeline.

use crate::error::Error as CrateError;
use thiserror::Error;

/// Error type for crawler operations
#[derive(Debug, Error)]
pub enum CrawlError {
    /// HTTP client error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// HTML parsing error
    #[error("HTML parsing error: {0}")]
    HtmlParse(String),

    /// Content extraction error
    #[error("Content extraction error: {0}")]
    ContentExtraction(String),

    /// Rate limit error
    #[error("Rate limit error: {0}")]
    RateLimit(String),

    /// Robots.txt error
    #[error("Robots.txt error: {0}")]
    RobotsTxt(String),

    /// URL parsing error
    #[error("URL parsing error: {0}")]
    UrlParse(#[from] url::ParseError),

    /// Other errors
    #[error("{0}")]
    Other(String),
}

impl From<CrawlError> for CrateError {
    fn from(err: CrawlError) -> Self {
        match err {
            CrawlError::Http(e) => CrateError::Http(e),
            CrawlError::UrlParse(e) => CrateError::Other(format!("URL parse error: {}", e)),
            _ => CrateError::Crawl(err.to_string()),
        }
    }
}
