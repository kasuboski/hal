//! Content extraction functionality for the crawler module

use crate::crawler::error::CrawlError;
use crate::crawler::PageMetadata;
use html2md::parse_html;
use scraper::{Html, Selector};
use tracing::warn;
use url::Url;

/// Clean HTML by removing unwanted elements and keeping only content elements
///
/// # Arguments
///
/// * `html` - The HTML to clean
/// * `content_selectors` - CSS selectors for content to include
/// * `exclude_selectors` - CSS selectors for elements to exclude
///
/// # Returns
///
/// The cleaned HTML as a string
pub fn clean_html(
    html: &str,
    content_selectors: &[String],
    exclude_selectors: &[String],
) -> Result<String, CrawlError> {
    let document = Html::parse_document(html);

    // Create a new HTML string to build our clean document
    let mut clean_html = String::new();

    // If content selectors are provided, use them
    // Otherwise, use the whole document minus excluded parts
    if !content_selectors.is_empty() {
        for selector_str in content_selectors {
            match Selector::parse(selector_str) {
                Ok(selector) => {
                    for element in document.select(&selector) {
                        clean_html.push_str(&element.html());
                    }
                }
                Err(e) => {
                    warn!("Failed to parse selector '{}': {}", selector_str, e);
                }
            }
        }
    } else {
        // Start with the whole document
        clean_html = html.to_string();

        // Then remove excluded elements
        // This is a simplified approach - in a real implementation,
        // we would need to properly parse and manipulate the HTML
        for selector_str in exclude_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                let mut elements_to_remove = Vec::new();
                for element in document.select(&selector) {
                    elements_to_remove.push(element.html());
                }

                for element_html in elements_to_remove {
                    // This is a naive approach and might not work for all cases
                    // A more robust solution would use a proper HTML manipulation library
                    if let Some(pos) = clean_html.find(&element_html) {
                        clean_html.replace_range(pos..pos + element_html.len(), "");
                    }
                }
            }
        }
    }

    Ok(clean_html)
}

/// Convert HTML to Markdown
///
/// # Arguments
///
/// * `html` - The HTML to convert
///
/// # Returns
///
/// The converted Markdown
pub fn html_to_markdown(html: &str) -> String {
    parse_html(html)
}

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
