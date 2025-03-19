//! Integration with spider library for web crawling

use spider::tokio;
use spider::website::Website;
use spider_utils::spider_transformations::transformation::content::{
    transform_content, ReturnFormat, TransformConfig,
};
use tracing::{debug, error, info, info_span, instrument};

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

    let mut website = Website::new(url);
    website
        .configuration
        .with_respect_robots_txt(config.respect_robots_txt)
        .with_user_agent(Some(&config.user_agent))
        .with_delay(config.rate_limit_ms)
        .with_depth(config.max_depth.try_into().unwrap_or(0))
        .with_limit(config.max_pages);

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
