//! # Spider Library Integration Module
//!
//! This module integrates with the Spider web crawling library to provide the core
//! crawling functionality for the RAG pipeline. It transforms raw HTML content into
//! structured, clean text suitable for further processing.
//!
//! ## Key Components
//!
//! - `crawl_website`: Main function to crawl a website with given configuration
//! - Integration with Spider library's async crawling capabilities
//! - Content transformation pipeline for HTML to Markdown conversion
//!
//! ## Features
//!
//! - Asynchronous crawling with Tokio runtime
//! - URL filtering with regex patterns
//! - Markdown conversion for cleaner text processing
//! - Readability-focused content extraction
//! - Quality filtering to skip low-value pages
//! - Structured logging and instrumentation
//! - Proper error propagation
//!
//! This module forms the foundation of the content acquisition phase in the
//! RAG pipeline, gathering the raw material that will be processed, embedded,
//! and indexed for retrieval.

use regex::Regex;
use spider::compact_str::CompactString;
use spider::tokio;
use spider::website::Website;
use spider_utils::spider_transformations::transformation::content::{
    ReturnFormat, TransformConfig, transform_content,
};
use tracing::{debug, error, info, info_span, instrument};
use url::Url;

use crate::crawler::content_extraction::extract_metadata;
use crate::crawler::error::CrawlError;
use crate::crawler::{CrawledPage, CrawlerConfig, PageMetadata};

/// Crawl a website and extract content
///
/// # Arguments
///
/// * `url` - The URL to crawl
/// * `config` - The crawler configuration
///
/// # Returns
///
/// A vector of crawled pages
#[instrument]
pub async fn crawl_website(
    url: &str,
    config: CrawlerConfig,
) -> Result<Vec<CrawledPage>, CrawlError> {
    info!("Starting crawl for {}", url);
    debug!("Crawler config: {:?}", config);

    let base_url = Url::parse(url)?;
    let base_path = regex::escape(base_url.path());
    let domain = base_url.host_str().ok_or(url::ParseError::EmptyHost)?;
    let domain = regex::escape(domain);
    let scheme = base_url.scheme();

    let allowed: Option<Vec<CompactString>> = if config.child_links_only {
        let regex_pattern_str = format!("^{scheme}://{domain}{base_path}.*");
        let _regex_pattern = Regex::new(&regex_pattern_str)
            .map_err(|e| CrawlError::Other(format!("Failed to create regex pattern: {}", e)))?;
        debug!("Using regex pattern: {}", regex_pattern_str);
        Some(vec![CompactString::from(&regex_pattern_str)])
    } else {
        None
    };

    let mut website = Website::new(url);
    website
        .configuration
        .with_respect_robots_txt(config.respect_robots_txt)
        .with_user_agent(Some(&config.user_agent))
        .with_delay(config.rate_limit_ms)
        .with_depth(config.max_depth.try_into().unwrap_or(0))
        .with_limit(config.max_pages)
        .with_whitelist_url(allowed);

    let mut rx = website
        .subscribe(10)
        .ok_or_else(|| CrawlError::Other("Failed to subscribe to website".to_string()))?;
    let handle = tokio::spawn(async move {
        let mut pages = Vec::new();
        while let Ok(page) = rx.recv().await {
            let _page_span = info_span!("process_page", url = %page.get_url());
            debug!("Received page: {}", page.get_url());

            let transform_config = TransformConfig {
                return_format: ReturnFormat::Markdown,
                readability: true,
                main_content: true,
                ..Default::default()
            };

            let markdown = transform_content(&page, &transform_config, &None, &None, &None);
            if markdown.len() < 100 {
                debug!("Skipping page: {}", page.get_url());
                continue;
            }
            let metadata_result = extract_metadata(page.get_url(), &page.get_html());

            match metadata_result {
                Ok(metadata) => {
                    let crawled_page = CrawledPage {
                        url: page.get_url().to_string(),
                        content: markdown,
                        metadata,
                    };
                    pages.push(crawled_page);
                }
                Err(e) => {
                    error!("Error extracting metadata: {:?}", e);
                    pages.push(CrawledPage {
                        url: page.get_url().to_string(),
                        content: markdown,
                        metadata: PageMetadata {
                            title: None,
                            description: None,
                            author: None,
                            publication_date: None,
                            domain: page.get_url().to_string(),
                        },
                    });
                }
            }
        }
        pages
    });

    website.crawl().await;
    info!("Crawl finished");
    website.unsubscribe();
    let pages = handle
        .await
        .map_err(|e| CrawlError::Other(format!("Task join error: {}", e)))?;
    info!("Processed {} pages", pages.len());
    Ok(pages)
}
