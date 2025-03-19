**Project: Migrating Custom Crawler to the `spider` Library**

**Goal:** Replace the existing custom web crawler implementation with the `spider` library to improve maintainability, efficiency, and leverage `spider`'s built-in features.

**Developer:** Trae

**Overall Strategy:**

We will incrementally replace the core functionality of the custom crawler with `spider`'s components.  We'll start with basic crawling and then integrate our existing content processing (Markdown conversion and metadata extraction).  We'll pay close attention to `tracing` instrumentation and ensure we don't hold spans across `.await` calls, which can lead to performance issues. The CrawlerConfig should more closely align to the Spider config options

**Phase 1: Setup and Basic Crawl**

**Step 1: Add Dependencies**

*   **Task:** Add `spider` and `spider_utils` to the project's dependencies.
*   Already Done

**Step 2: Basic `spider` Integration**

*   **Task:** Replace the core crawling loop with a basic `spider` crawl.
*   **Instructions:**
    1.  Open `src/crawler/spider_integration.rs`.
    2.  Replace the existing `crawl_website` function with the following code:

        ```rust
        use spider::tokio;
        use spider::website::Website;
        use tracing::{instrument, info};
        use crate::crawler::{CrawledPage, CrawlerConfig, CrawlError}; // Adjust as needed

        #[instrument]
        pub async fn crawl_website(
            url: &str,
            config: CrawlerConfig,
        ) -> Result<Vec<CrawledPage>, CrawlError> {

            info!("Starting crawl for {}", url);

            let mut website = Website::new(url);
            website
                .configuration
                .with_respect_robots_txt(config.respect_robots_txt)
                .with_user_agent(Some(config.user_agent))
                .with_delay(config.rate_limit_ms);

            // Basic crawl - we'll add more configuration later.
            website.crawl().await;
            info!("Crawl finished, gathering results.");

            // Placeholder - we'll fill this in later.
            Ok(vec![])
        }
        ```

    3.  **Explanation:**
        *   We create a `spider::website::Website` instance.
        *   We configure it with:
            *   `respect_robots_txt` from our existing `CrawlerConfig`.
            *   `user_agent` from our existing `CrawlerConfig`.
            *   `delay` (rate limiting) from our existing `CrawlerConfig`.
        *   `website.crawl().await` performs the basic crawl. This replaces our custom queue, request handling, and basic link extraction.

**Step 3: Initial Testing**

*   **Task:**  Run a basic test to ensure `spider` is working.
*   **Instructions:**
    1.  Modify your main application code (where `crawl_website` is called) to use the new `spider_integration.rs`.  You'll likely need to adjust how you pass the `CrawlerConfig`.
    2.  Run the application with a test URL.
    3.  Verify that the application runs without errors and that basic crawling occurs (you should see log messages from `tracing`).  You won't see any page content yet, as we haven't integrated the processing.

**Phase 2: Content Processing and Metadata Extraction**

**Step 4:  Subscribe to Crawled Pages**

*   **Task:** Use `spider`'s subscription mechanism to receive pages as they are crawled.
*   **Instructions:**
    1.  Modify the `crawl_website` function in `src/crawler/spider_integration.rs` as follows:

        ```rust
        // src/crawler/spider_integration.rs
        use spider::tokio;
        use spider::website::Website;
        use spider_utils::spider_transformations::transformation::content::{TransformConfig, transform_content, ReturnFormat};
        use tracing::{instrument, info, debug, error};
        use crate::crawler::{CrawledPage, CrawlerConfig, CrawlError, PageMetadata}; // Adjust
        use crate::crawler::content_extraction::extract_metadata;
        use spider::error::WebsiteError;

        #[instrument]
        pub async fn crawl_website(
            url: &str,
            config: CrawlerConfig,
        ) -> Result<Vec<CrawledPage>, CrawlError> {

            info!("Starting crawl for {}", url);

            let mut website = Website::new(url);
            website
                .configuration
                .with_respect_robots_txt(config.respect_robots_txt)
                .with_user_agent(Some(config.user_agent))
                .with_delay(config.rate_limit_ms)
                .with_depth(config.max_depth)
                .with_limit(config.max_pages as usize);

            let mut rx = website.subscribe(10).unwrap();
            let mut pages = Vec::new();

            let handle = tokio::spawn(async move {
                while let Ok(page) = rx.recv().await {
                    // Create a span for each page, but *don't* hold it across the await.
                    debug!("Received page: {}", page.get_url());

                    let mut transform_config = TransformConfig::default();
                    transform_config.return_format = ReturnFormat::Markdown;
                    transform_config.readability = true;

                    // Perform the transformation *without* holding the span across the await.
                    let markdown = transform_content(&page, &transform_config, &None, &None, &None);

                    // Same for metadata extraction - do it outside the span that might be held.
                    let metadata_result = extract_metadata(&page.get_url(), &page.get_html());

                    // Now, create a short-lived span for logging and processing the result.
                    {
                        let _page_span = info_span!("process_page", url = %page.get_url());

                        match metadata_result {
                            Ok(metadata) => {
                                let crawled_page = CrawledPage {
                                    url: page.get_url().to_string(),
                                    content: markdown,
                                    metadata,
                                };
                                pages.push(crawled_page);

                            },
                            Err(e) => {
                                error!("Error extracting metadata: {:?}", e);
                            }
                        }
                    }
                }
            });
            website.crawl().await;
            info!("Crawl finished");
            website.unsubscribe();
            let _ = handle.await;
            info!("Processed {} pages", pages.len());
             match website.get_error() {
                Some(e) => match e {
                    WebsiteError::Crawl(ce) => match ce.kind {
                        // TODO: refine error handling.
                        spider::error::ErrorKind::Content(_) => Err(CrawlError::ContentExtraction("Content Extraction Failed.".to_string())),
                        _ =>  Err(CrawlError::Other(format!("Spider internal error: {:?}", e))),
                    },
                    _ => Err(CrawlError::Other(format!("Spider internal error: {:?}", e))),
                },
                None => (),
            }
            Ok(pages)
        }
        ```
    2.  **Explanation:**
        *   `website.subscribe(10)`:  We subscribe to the stream of crawled pages. `10` is a buffer size; adjust as needed.
        *   `tokio::spawn`:  We use `tokio::spawn` to process pages concurrently *without* blocking the main crawl loop.  This is crucial for performance.
        *   `rx.recv().await`: Inside the spawned task, we receive pages from the subscription.
        *   **`TransformConfig` and `transform_content`:**  We use `spider_utils` to convert the HTML to Markdown.  This replaces our old `clean_html` and `html_to_markdown` functions.
        *   **`extract_metadata`:** We reuse our existing `extract_metadata` function.
        *   **Span Management:**
            *   We log "Received page" *before* any `.await` calls.
            *   `transform_content` and `extract_metadata` are called *without* a span held across them.
            *   A *short-lived* span (`_page_span`) is created *after* the `await` calls, only for logging and result processing.
        * **Error Handling** Error handling is implemented for website errors.

**Step 5: Integrate Depth and Page Limits**

*   **Task:**  Apply the `max_depth` and `max_pages` settings from `CrawlerConfig`.
*   **Instructions:**
    1.  In `crawl_website`, add the following lines to the `website` configuration:

        ```rust
        // ... (within the crawl_website function) ...
        website
            .configuration
            // ... (other configurations) ...
            .with_depth(config.max_depth)  // Add this line
            .with_limit(config.max_pages as usize); // Add this line
        ```

    2.  **Explanation:** `spider` has built-in support for these limits, so we just need to configure them.

**Step 6: Remove Old Code**

*   **Task:**  Remove the now-unnecessary code from the original crawler.
*   **Instructions:**
    1.  Carefully remove the following from `src/crawler/spider_integration.rs` (and any other relevant files):
        *   The `normalize_url` function (spider handles this).
        *   The `queue`, `visited`, and `semaphore` variables and logic (spider manages these).
        *   The custom `reqwest` request handling code.
        *   The `clean_html` and `html_to_markdown` functions from the codebase.
    2.  **Important:**  Do this carefully and incrementally, testing frequently.  It's easy to accidentally remove something important.

**Step 7:  Testing and Refinement**

*   **Task:**  Thoroughly test the new `spider`-based crawler.
*   **Instructions:**
    1.  Run the crawler with various configurations:
        *   Different `max_depth` and `max_pages` values.
        *   Different websites.
        *   With and without `respect_robots_txt` enabled.
    2.  Compare the output (crawled pages, metadata) with the original crawler's output to ensure correctness.
    3.  **Refine Error Handling:** Add more specific error handling for different `spider` error types if needed.  Consider how you want to handle errors during metadata extraction or Markdown conversion (log and continue, stop the crawl, etc.).
    4.  **Adjust Buffer Sizes:** Experiment with the buffer size in `website.subscribe()` to find the optimal value for your use case.
    5. **Explore Advanced Features:** If needed, investigate `spider`'s more advanced features (blacklists, whitelists, custom request headers, etc.).

**Final Code (Illustrative):**
This is what your final `src/crawler/spider_integration.rs` might look like:

```rust
// src/crawler/spider_integration.rs

use spider::tokio;
use spider::website::Website;
use spider_utils::spider_transformations::transformation::content::{TransformConfig, transform_content, ReturnFormat};
use tracing::{instrument, info, debug, error, info_span};
use crate::crawler::{CrawledPage, CrawlerConfig, CrawlError, PageMetadata};
use crate::crawler::content_extraction::extract_metadata;
use spider::error::WebsiteError;


#[instrument]
pub async fn crawl_website(
    url: &str,
    config: CrawlerConfig,
) -> Result<Vec<CrawledPage>, CrawlError> {

    info!("Starting crawl for {}", url);

    let mut website = Website::new(url);
    website
        .configuration
        .with_respect_robots_txt(config.respect_robots_txt)
        .with_user_agent(Some(config.user_agent))
        .with_delay(config.rate_limit_ms)
        .with_depth(config.max_depth)
        .with_limit(config.max_pages as usize);

    let mut rx = website.subscribe(10).unwrap();
    let mut pages = Vec::new();

    let handle = tokio::spawn(async move {
        while let Ok(page) = rx.recv().await {
            debug!("Received page: {}", page.get_url());

            let mut transform_config = TransformConfig::default();
            transform_config.return_format = ReturnFormat::Markdown;
            transform_config.readability = true;

            let markdown = transform_content(&page, &transform_config, &None, &None, &None);
            let metadata_result = extract_metadata(&page.get_url(), &page.get_html());

            {
                let _page_span = info_span!("process_page", url = %page.get_url());
                match metadata_result {
                    Ok(metadata) => {
                        let crawled_page = CrawledPage {
                            url: page.get_url().to_string(),
                            content: markdown,
                            metadata,
                        };
                        pages.push(crawled_page);
                    },
                    Err(e) => {
                        error!("Error extracting metadata: {:?}", e);
                    }
                }
            }
        }
    });

    website.crawl().await;
	info!("Crawl finished");
    website.unsubscribe();
    let _ = handle.await; // Wait for processing to complete
	info!("Processed {} pages", pages.len());
    match website.get_error() {
        Some(e) => match e {
            WebsiteError::Crawl(ce) => match ce.kind {
                // TODO: refine error handling.
                spider::error::ErrorKind::Content(_) => Err(CrawlError::ContentExtraction("Content Extraction Failed.".to_string())),
                _ =>  Err(CrawlError::Other(format!("Spider internal error: {:?}", e))),
            },
            _ => Err(CrawlError::Other(format!("Spider internal error: {:?}", e))),
        },
        None => (),
    }
    Ok(pages)
}
```

**Key Reminders for the Junior Developer:**

*   **Read the `spider` Documentation:**  The `spider` library has excellent documentation.  Encourage the developer to refer to it for details on configuration options and advanced features.
*   **Test Frequently:**  Testing after each step is crucial to catch errors early and ensure a smooth migration.
*   **Incremental Changes:**  The step-by-step approach makes the task less daunting and easier to debug.
*   **Understand `tracing`:**  Explain the importance of `tracing` for debugging and monitoring, and emphasize the correct way to use spans with `async` code.
*   **Ask Questions:** Encourage them to ask questions if they get stuck or are unsure about anything.
*   **Small Functions** Create small functions for better readability. This also makes it easier to use the #[instrument] tracing macro for tracing all aspects of the app.
*   **Document** Document the steps and decisions you make as you migrate in `docs/developer.md`. Remember to follow this plan and ask for clarification if you need to deviate.

