//! # Crawler Configuration Module
//!
//! This module provides configuration options for the web crawler, including
//! controls for crawl depth, rate limiting, and content selection. It uses a
//! builder pattern for flexible configuration.
//!
//! ## Key Components
//!
//! - `CrawlerConfig`: The main configuration struct with crawler parameters
//! - `CrawlerConfigBuilder`: Builder pattern implementation for easier configuration
//!
//! ## Features
//!
//! - Default configurations suitable for polite crawling
//! - Fine-grained control over crawl behavior (depth, pages, rate limits)
//! - Content selection via CSS selectors
//! - Exclusion patterns for boilerplate content (navigation, headers, footers)
//! - User-agent customization

use std::time::Duration;

/// Configuration for the crawler
#[derive(Debug, Clone)]
pub struct CrawlerConfig {
    /// Maximum depth to crawl
    pub max_depth: u32,

    /// Maximum number of pages to crawl
    pub max_pages: u32,

    /// Rate limit in milliseconds between requests
    pub rate_limit_ms: u64,

    /// Whether to respect robots.txt
    pub respect_robots_txt: bool,

    /// Whether to only crawl links underneath the initial URL
    pub child_links_only: bool,

    /// User agent to use for requests
    pub user_agent: String,

    /// CSS selectors for content to include
    pub content_selectors: Vec<String>,

    /// CSS selectors for elements to exclude
    pub exclude_selectors: Vec<String>,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            max_depth: 2,
            max_pages: 100,
            rate_limit_ms: 500,
            respect_robots_txt: true,
            child_links_only: true,
            user_agent: format!("hal-crawler/{}", env!("CARGO_PKG_VERSION")),
            content_selectors: Vec::new(),
            exclude_selectors: vec![
                "nav".to_string(),
                "header".to_string(),
                "footer".to_string(),
                "aside".to_string(),
                ".navigation".to_string(),
                ".menu".to_string(),
                ".sidebar".to_string(),
                ".ads".to_string(),
                ".comments".to_string(),
                "#nav".to_string(),
                "#header".to_string(),
                "#footer".to_string(),
                "#sidebar".to_string(),
                "#comments".to_string(),
            ],
        }
    }
}

/// Builder for CrawlerConfig
#[derive(Debug, Default)]
pub struct CrawlerConfigBuilder {
    config: CrawlerConfig,
}

impl CrawlerConfigBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: CrawlerConfig::default(),
        }
    }

    /// Set the maximum depth to crawl
    pub fn max_depth(mut self, max_depth: u32) -> Self {
        self.config.max_depth = max_depth;
        self
    }

    /// Set the maximum number of pages to crawl
    pub fn max_pages(mut self, max_pages: u32) -> Self {
        self.config.max_pages = max_pages;
        self
    }

    /// Set the rate limit in milliseconds between requests
    pub fn rate_limit_ms(mut self, rate_limit_ms: u64) -> Self {
        self.config.rate_limit_ms = rate_limit_ms;
        self
    }

    /// Set whether to respect robots.txt
    pub fn respect_robots_txt(mut self, respect_robots_txt: bool) -> Self {
        self.config.respect_robots_txt = respect_robots_txt;
        self
    }

    /// Set the user agent to use for requests
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.config.user_agent = user_agent.into();
        self
    }

    /// Set the CSS selectors for content to include
    pub fn content_selectors(mut self, content_selectors: Vec<String>) -> Self {
        self.config.content_selectors = content_selectors;
        self
    }

    /// Set the CSS selectors for elements to exclude
    pub fn exclude_selectors(mut self, exclude_selectors: Vec<String>) -> Self {
        self.config.exclude_selectors = exclude_selectors;
        self
    }

    /// Build the configuration
    pub fn build(self) -> CrawlerConfig {
        self.config
    }
}

impl CrawlerConfig {
    /// Create a new builder
    pub fn builder() -> CrawlerConfigBuilder {
        CrawlerConfigBuilder::new()
    }

    /// Get the rate limit as a Duration
    pub fn rate_limit(&self) -> Duration {
        Duration::from_millis(self.rate_limit_ms)
    }
}
