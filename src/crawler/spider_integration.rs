//! Integration with reqwest for web crawling

use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tracing::{debug, info, info_span, instrument, warn};
use url::Url;

use crate::crawler::content_extraction::{clean_html, extract_metadata, html_to_markdown};
use crate::crawler::error::CrawlError;
use crate::crawler::{CrawledPage, CrawlerConfig};

/// Normalize a URL by removing query parameters and fragments
///
/// This ensures that URLs like "example.com/page" and "example.com/page#section1"
/// are treated as the same page.
fn normalize_url(url: &Url) -> String {
    let mut normalized = url.clone();
    normalized.set_query(None);
    normalized.set_fragment(None);

    // Normalize trailing slashes
    let mut path = normalized.path().to_string();

    // Remove trailing slash if it's not the root path
    if path.len() > 1 && path.ends_with('/') {
        path.pop();
        normalized.set_path(&path);
    }

    normalized.to_string()
}

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
    info!("Crawling website: {}", url);
    debug!("Crawler config: {:?}", config);

    // Parse the URL to ensure it's valid
    let base_url = Url::parse(url).map_err(CrawlError::UrlParse)?;

    // Create a reqwest client
    let client = Client::builder()
        .user_agent(&config.user_agent)
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| CrawlError::Other(format!("Failed to create HTTP client: {}", e)))?;

    // Create a semaphore to limit concurrent requests
    let semaphore = Semaphore::new(5); // Limit to 5 concurrent requests

    // Create a queue of URLs to crawl
    let mut queue = VecDeque::new();
    queue.push_back((base_url.clone(), 0)); // Start with the base URL at depth 0

    // Create a set of visited URLs
    let mut visited = HashSet::new();
    visited.insert(normalize_url(&base_url));

    // Create a vector to store crawled pages
    let mut pages = Vec::new();

    // Process the queue
    while let Some((url, depth)) = queue.pop_front() {
        // Check if we've reached the maximum number of pages
        if pages.len() >= config.max_pages as usize {
            info!("Reached maximum number of pages ({})", config.max_pages);
            break;
        }

        // Check if we've reached the maximum depth
        if depth > config.max_depth {
            debug!("Skipping {} (depth {})", url, depth);
            continue;
        }

        // Acquire a permit from the semaphore
        let permit = semaphore.acquire().await.unwrap();

        // Fetch the page
        info!("Fetching {}", url);
        let response = match client.get(url.clone()).send().await {
            Ok(response) => response,
            Err(e) => {
                warn!("Failed to fetch {}: {}", url, e);
                drop(permit);
                continue;
            }
        };

        // Check if the response is successful
        if !response.status().is_success() {
            warn!("Failed to fetch {}: {}", url, response.status());
            drop(permit);
            continue;
        }

        // Get the HTML content
        let html = match response.text().await {
            Ok(html) => html,
            Err(e) => {
                warn!("Failed to get HTML from {}: {}", url, e);
                drop(permit);
                continue;
            }
        };

        {
            let process = info_span!("process_url", url = %url);
            // Clean the HTML
            let clean_html =
                match clean_html(&html, &config.content_selectors, &config.exclude_selectors) {
                    Ok(clean_html) => clean_html,
                    Err(e) => {
                        warn!("Failed to clean HTML from {}: {}", url, e);
                        drop(permit);
                        continue;
                    }
                };

            // Convert to Markdown
            let markdown = html_to_markdown(&clean_html);

            // Extract metadata
            let metadata = match extract_metadata(url.as_ref(), &html) {
                Ok(metadata) => metadata,
                Err(e) => {
                    warn!("Failed to extract metadata from {}: {}", url, e);
                    drop(permit);
                    continue;
                }
            };

            // Create a crawled page
            let page = CrawledPage {
                url: url.to_string(),
                content: markdown,
                metadata,
            };

            // Add the page to the list
            pages.push(page);

            // Extract links from the page
            if depth < config.max_depth {
                let document = Html::parse_document(&html);
                let selector = match Selector::parse("a[href]") {
                    Ok(selector) => selector,
                    Err(e) => {
                        warn!("Failed to parse selector: {}", e);
                        drop(permit);
                        continue;
                    }
                };

                for element in document.select(&selector) {
                    if let Some(href) = element.value().attr("href") {
                        // Parse the URL
                        let next_url = match url.join(href) {
                            Ok(url) => url,
                            Err(e) => {
                                debug!("Failed to parse URL {}: {}", href, e);
                                continue;
                            }
                        };

                        // Check if the URL is from the same domain
                        if next_url.host_str() != base_url.host_str() {
                            debug!("Skipping external URL: {}", next_url);
                            continue;
                        }

                        // Normalize the URL by removing query parameters and fragments
                        let normalized_url = normalize_url(&next_url);

                        // Check if we've already visited this URL
                        if visited.contains(&normalized_url) {
                            debug!("Skipping already visited URL: {}", next_url);
                            continue;
                        }

                        // Add the URL to the queue
                        debug!("Adding URL to queue: {}", next_url);
                        queue.push_back((next_url.clone(), depth + 1));
                        visited.insert(normalized_url);
                    }
                }
            }

            // Release the permit
            drop(permit);
            drop(process);
        }

        // Add a delay to avoid overwhelming the server
        sleep(Duration::from_millis(config.rate_limit_ms)).await;
    }

    info!("Crawled {} pages", pages.len());
    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url() {
        // Test cases with different URL formats
        let test_cases = vec![
            // Basic URL without query or fragment
            ("https://example.com/page", "https://example.com/page"),
            // URL with query parameters
            (
                "https://example.com/page?param=value",
                "https://example.com/page",
            ),
            // URL with fragment
            (
                "https://example.com/page#section1",
                "https://example.com/page",
            ),
            // URL with both query parameters and fragment
            (
                "https://example.com/page?param=value#section1",
                "https://example.com/page",
            ),
            // URL with multiple query parameters
            (
                "https://example.com/page?param1=value1&param2=value2",
                "https://example.com/page",
            ),
            // URL with path and trailing slash (should be normalized)
            (
                "https://example.com/path/to/page/",
                "https://example.com/path/to/page",
            ),
            // URL with port
            (
                "https://example.com:8080/page?param=value#section",
                "https://example.com:8080/page",
            ),
            // Root URL with trailing slash (should keep the slash as it's the root)
            ("https://example.com/", "https://example.com/"),
            // Root URL without trailing slash (should be the same as with slash)
            ("https://example.com", "https://example.com/"),
            // Nested path with trailing slash
            (
                "https://example.com/path/to/nested/",
                "https://example.com/path/to/nested",
            ),
            // Same nested path without trailing slash
            (
                "https://example.com/path/to/nested",
                "https://example.com/path/to/nested",
            ),
        ];

        for (input, expected) in test_cases {
            let url = Url::parse(input).unwrap();
            let normalized = normalize_url(&url);
            assert_eq!(normalized, expected, "Failed to normalize URL: {}", input);
        }
    }
}
