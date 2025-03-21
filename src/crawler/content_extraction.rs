//! # Content Extraction Module
//!
//! This module provides utilities for extracting structured content and metadata
//! from HTML pages. It's responsible for parsing HTML documents and extracting
//! valuable information for the RAG pipeline.
//!
//! ## Key Components
//!
//! - `extract_metadata`: Extracts standard metadata from HTML (title, description, etc.)
//!
//! ## Features
//!
//! - Robust HTML parsing using the scraper library
//! - Extraction of common metadata fields from web pages
//! - Domain extraction for source attribution
//! - Error handling for malformed HTML or missing data
//!
//! The extracted data becomes part of the document metadata in the RAG system,
//! which can be used for filtering, ranking, and providing context to the LLM.

use crate::crawler::error::CrawlError;
use crate::crawler::PageMetadata;
use scraper::{Html, Selector};
use url::Url;

/// Extract metadata from a page
///
/// # Arguments
///
/// * `url` - The URL of the page
/// * `html` - The HTML of the page
///
/// # Returns
///
/// The extracted metadata
pub fn extract_metadata(url: &str, html: &str) -> Result<PageMetadata, CrawlError> {
    let document = Html::parse_document(html);

    // Parse URL to extract domain
    let parsed_url = Url::parse(url).map_err(CrawlError::UrlParse)?;

    let domain = parsed_url
        .host_str()
        .ok_or_else(|| CrawlError::Other("Failed to extract domain from URL".to_string()))?
        .to_string();

    // Extract title
    let title_selector = Selector::parse("title")
        .map_err(|e| CrawlError::HtmlParse(format!("Failed to parse title selector: {}", e)))?;

    let title = document
        .select(&title_selector)
        .next()
        .map(|element| element.text().collect::<String>());

    // Extract description
    let description_selector = Selector::parse("meta[name='description']").map_err(|e| {
        CrawlError::HtmlParse(format!("Failed to parse description selector: {}", e))
    })?;

    let description = document
        .select(&description_selector)
        .next()
        .and_then(|element| element.value().attr("content"))
        .map(|s| s.to_string());

    // Extract publication date
    let publication_date = None; // This would require more complex logic to extract reliably

    // Extract author
    let author_selector = Selector::parse("meta[name='author']")
        .map_err(|e| CrawlError::HtmlParse(format!("Failed to parse author selector: {}", e)))?;

    let author = document
        .select(&author_selector)
        .next()
        .and_then(|element| element.value().attr("content"))
        .map(|s| s.to_string());

    Ok(PageMetadata {
        title,
        description,
        publication_date,
        author,
        domain,
    })
}
