# Feature Implementation Plan: Website Crawling, Indexing, and Search for RAG in Rust

## Overview
This feature adds CLI commands to crawl websites, index their content for Retrieval-Augmented Generation (RAG), and provide search functionality. The system will embed text chunks, generate summaries and context strings using a small LLM, and store website metadata including last index date.

## Tech Stack
- **Language & Runtime**: Rust with Tokio for async operations
- **CLI Framework**: Clap for command-line parsing
- **Web Crawler**: Spider crate for website crawling
- **Content Processing**: HTML to Markdown conversion
- **Storage**: LibSQL (SQLite variant) with vector search capabilities
- **Async**: Tokio runtime for asynchronous operations

## Implementation Steps

### 1. Website Crawler Component

#### 1.1 Integration with Spider Crate
- Configure Spider crate with appropriate settings
- Implement Tokio-compatible asynchronous crawling
- Add configuration options for depth, rate limiting, and filters
- Handle content conversion from HTML to Markdown

#### 1.2 Content Cleaning
- Remove irrelevant elements (navigation, ads, footers, etc.)
- Implement selectors to target and exclude non-content elements
- Preserve important semantic structure
- Configure content extraction rules per domain if needed

#### 1.3 Crawler Configuration
- Define configuration struct for crawler settings
- Implement builder pattern for flexible configuration
- Support URL patterns for inclusion/exclusion

#### 1.4 Content Extraction
- Extract useful content from crawled pages
- Convert HTML to clean Markdown using the identified library
- Preserve metadata (title, publication date, etc.)

#### Pseudocode:
```rust
struct CrawlerConfig {
    max_depth: u32,
    max_pages: u32,
    rate_limit_ms: u64,
    respect_robots_txt: bool,
    user_agent: String,
    content_selectors: Vec<String>,  // CSS selectors for content
    exclude_selectors: Vec<String>,  // CSS selectors for elements to remove
}

async fn crawl_website(url: &str, config: CrawlerConfig) -> Result<Vec<CrawledPage>, CrawlError> {
    let spider = Spider::builder()
        .max_depth(config.max_depth)
        .request_interval(Duration::from_millis(config.rate_limit_ms))
        .respect_robots_txt(config.respect_robots_txt)
        .user_agent(&config.user_agent)
        .build();
    
    let mut results = Vec::new();
    let mut pages_processed = 0;
    
    let response_stream = spider.crawl(url).await?;
    
    tokio::pin!(response_stream);
    
    while let Some(response) = response_stream.next().await {
        if pages_processed >= config.max_pages {
            break;
        }
        
        match response {
            Ok(page) => {
                // Clean HTML before conversion
                let clean_html = clean_html(
                    &page.body, 
                    &config.content_selectors, 
                    &config.exclude_selectors
                )?;
                
                let markdown = html_to_markdown(&clean_html);
                let metadata = extract_metadata(&page);
                
                results.push(CrawledPage {
                    url: page.url.to_string(),
                    content: markdown,
                    metadata,
                });
                
                pages_processed += 1;
            },
            Err(e) => {
                eprintln!("Error crawling page: {}", e);
                // Log error but continue crawling
            }
        }
    }
    
    Ok(results)
}

fn clean_html(
    html: &str, 
    content_selectors: &[String], 
    exclude_selectors: &[String]
) -> Result<String, HtmlCleanError> {
    let document = Html::parse_document(html);
    
    // Create a new document with only content we want
    let mut clean_document = Html::parse_document("<html><body></body></html>");
    let body_selector = Selector::parse("body").unwrap();
    let body = clean_document.select(&body_selector).next().unwrap();
    
    // If content selectors are provided, use them
    // Otherwise, use the whole document minus excluded parts
    if !content_selectors.is_empty() {
        for selector_str in content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    body.append_child(element.clone());
                }
            }
        }
    } else {
        // Copy the whole document
        body.append_child(document.clone());
        
        // Then remove excluded elements
        for selector_str in exclude_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in clean_document.select(&selector) {
                    element.remove();
                }
            }
        }
    }
    
    // Default excluded elements (navigation, ads, etc.)
    let default_excludes = [
        "nav", "header", "footer", "aside", 
        ".navigation", ".menu", ".sidebar", ".ads", ".comments",
        "#nav", "#header", "#footer", "#sidebar", "#comments"
    ];
    
    for selector_str in default_excludes.iter() {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in clean_document.select(&selector) {
                element.remove();
            }
        }
    }
    
    Ok(clean_document.html())
}
```

#### Acceptance Criteria:
- Spider crate is properly integrated with async Tokio runtime
- Navigation elements, ads, and other non-content elements are successfully removed
- HTML is successfully converted to clean Markdown
- Crawler respects configuration options (depth, rate limits)
- Errors are handled gracefully without stopping the entire crawl process

### 2. Content Processor Component

#### 2.1 Text Chunking
- Implement Markdown-aware text chunking algorithm
- Preserve headings and structural elements when possible
- Balance chunk size for optimal embedding and LLM processing

#### 2.2 Embedding Generation
- Integrate with embedding model (e.g., via REST API or local model)
- Generate embeddings for each chunk
- Implement batching for efficiency

#### 2.3 LLM Integration for Summaries and Context
- Create client for small LLM API
- Design prompts for summary generation
- Design prompts for context string extraction
- Implement retry and error handling mechanisms

#### Pseudocode:
```rust
struct ChunkOptions {
    target_chunk_size: usize,
    overlap_size: usize,
}

struct ProcessorOptions {
    chunk_options: ChunkOptions,
    llm_model: String,
    embedding_dimensions: usize,
}

async fn process_content(
    page: CrawledPage, 
    options: ProcessorOptions
) -> Result<Vec<ProcessedChunk>, ProcessError> {
    // Chunk the markdown content
    let chunks = chunk_markdown(&page.content, &options.chunk_options);
    let mut processed_chunks = Vec::new();
    
    // Process chunks in parallel with bounded concurrency
    let semaphore = Arc::new(Semaphore::new(5)); // Limit concurrent API calls
    
    let tasks: Vec<_> = chunks.into_iter().map(|chunk| {
        let permit = semaphore.clone().acquire_owned();
        let llm_model = options.llm_model.clone();
        let metadata = page.metadata.clone();
        let url = page.url.clone();
        
        tokio::spawn(async move {
            let _permit = permit.await?;
            
            // Generate embedding
            let embedding = generate_embedding(&chunk.text).await?;
            
            // Generate summary using LLM
            let summary = generate_summary(&chunk.text, &llm_model).await?;
            
            // Generate context string using LLM
            let context = generate_context_string(&chunk.text, &url, &metadata, &llm_model).await?;
            
            Ok::<ProcessedChunk, ProcessError>(ProcessedChunk {
                text: chunk.text,
                embedding,
                summary,
                context,
                metadata: ChunkMetadata {
                    source_url: url,
                    position: chunk.position,
                    heading: chunk.heading,
                }
            })
        })
    }).collect();
    
    for task in futures::future::join_all(tasks).await {
        match task {
            Ok(Ok(processed_chunk)) => processed_chunks.push(processed_chunk),
            Ok(Err(e)) => eprintln!("Error processing chunk: {}", e),
            Err(e) => eprintln!("Task failed: {}", e),
        }
    }
    
    Ok(processed_chunks)
}
```

#### Acceptance Criteria:
- Content is chunked preserving markdown structure
- Embeddings are generated correctly for each chunk
- Summaries capture key information from chunks
- Context strings provide relevant additional context
- Processing handles errors gracefully with retries

### 3. Index Manager Component

#### 3.1 LibSQL Schema Design
- Design SQLite schema for websites, chunks, embeddings, etc.
- Implement vector search capabilities using LibSQL
- Create migration scripts for schema setup and updates

#### 3.2 Database Operations
- Implement CRUD operations for websites and chunks
- Create batch operations for efficient processing
- Implement efficient vector search queries

#### 3.3 Website Metadata Management
- Create structures for tracking website metadata
- Implement logic for updating last crawl date
- Store statistics about indexed content

#### Pseudocode:
```rust
struct Database {
    conn: libsql::Connection,
}

impl Database {
    async fn new(path: &str) -> Result<Self, DbError> {
        let conn = libsql::Connection::open(path).await?;
        
        // Run migrations if needed
        Self::run_migrations(&conn).await?;
        
        Ok(Self { conn })
    }
    
    async fn run_migrations(&self) -> Result<(), DbError> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS websites (
                id INTEGER PRIMARY KEY,
                url TEXT NOT NULL UNIQUE,
                domain TEXT NOT NULL,
                first_index_date INTEGER NOT NULL,
                last_index_date INTEGER NOT NULL,
                page_count INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS chunks (
                id INTEGER PRIMARY KEY,
                website_id INTEGER NOT NULL,
                url TEXT NOT NULL,
                text TEXT NOT NULL,
                summary TEXT NOT NULL,
                context TEXT NOT NULL,
                embedding BLOB NOT NULL,
                position INTEGER NOT NULL,
                heading TEXT,
                FOREIGN KEY (website_id) REFERENCES websites(id)
            );
            
            -- Add vector index for embeddings
            CREATE VIRTUAL TABLE IF NOT EXISTS chunk_vectors USING vector(
                embedding_data(384) -- Adjust dimension to match your embeddings
            );
        ").await?;
        
        Ok(())
    }
    
    async fn update_website_index(
        &self, 
        url: &str, 
        chunks: Vec<ProcessedChunk>
    ) -> Result<i64, DbError> {
        // Use a transaction for atomic updates
        let mut tx = self.conn.begin().await?;
        
        // Insert or update website
        let website_id = self.upsert_website(&mut tx, url, chunks.len()).await?;
        
        // Remove old chunks for this website
        self.remove_chunks_by_website(&mut tx, website_id).await?;
        
        // Insert new chunks
        self.insert_chunks(&mut tx, website_id, chunks).await?;
        
        // Commit transaction
        tx.commit().await?;
        
        Ok(website_id)
    }
    
    // Additional methods for CRUD operations...
}
```

#### Acceptance Criteria:
- LibSQL schema properly stores website metadata, chunks, embeddings, etc.
- Vector search is efficiently implemented
- Database operations handle concurrency appropriately
- Website metadata is updated correctly

### 4. Search System Component

#### 4.1 Vector Search Implementation
- Use LibSQL's vector search capabilities
- Implement filtering by website source
- Support various search parameters

#### 4.2 Search Result Processing
- Format search results with relevant metadata
- Implement scoring and ranking system
- Support pagination for large result sets

#### Pseudocode:
```rust
struct SearchOptions {
    limit: usize,
    min_score: f32,
    source_filter: Option<String>,
    date_range: Option<(i64, i64)>,
}

async fn search_index(
    db: &Database,
    query: &str,
    options: SearchOptions
) -> Result<Vec<SearchResult>, SearchError> {
    // Generate embedding for query
    let query_embedding = generate_embedding(query).await?;
    
    // Build SQL query with appropriate filters
    let mut sql = String::from("
        SELECT 
            c.id, c.text, c.summary, c.context, c.url,
            w.url as website_url, w.domain as website_domain,
            vector_distance(c.embedding, ?) as score
        FROM chunks c
        JOIN websites w ON c.website_id = w.id
        WHERE score <= ?
    ");
    
    let mut params: Vec<libsql::Value> = vec![
        libsql::Value::Blob(query_embedding.to_vec()),
        libsql::Value::Real(options.min_score as f64),
    ];
    
    // Add source filter if specified
    if let Some(source) = options.source_filter {
        sql.push_str(" AND w.domain = ?");
        params.push(libsql::Value::Text(source));
    }
    
    // Add date range filter if specified
    if let Some((start, end)) = options.date_range {
        sql.push_str(" AND w.last_index_date BETWEEN ? AND ?");
        params.push(libsql::Value::Integer(start));
        params.push(libsql::Value::Integer(end));
    }
    
    // Add order by and limit
    sql.push_str(" ORDER BY score ASC LIMIT ?");
    params.push(libsql::Value::Integer(options.limit as i64));
    
    // Execute query
    let mut stmt = db.conn.prepare(&sql).await?;
    let rows = stmt.query(&params).await?;
    
    // Process results
    let mut results = Vec::new();
    while let Some(row) = rows.next().await? {
        results.push(SearchResult {
            chunk_id: row.get(0)?,
            text: row.get(1)?,
            summary: row.get(2)?,
            context: row.get(3)?,
            url: row.get(4)?,
            website_url: row.get(5)?,
            website_domain: row.get(6)?,
            score: row.get(7)?,
        });
    }
    
    Ok(results)
}
```

#### Acceptance Criteria:
- Search returns relevant results based on embedding similarity
- Source filtering works correctly
- Results include all necessary metadata
- Search performance is acceptable with large indices

### 5. CLI Interface Component

#### 5.1 Clap Command Structure
- Define command structure using Clap
- Implement subcommands for crawl, index, search, etc.
- Provide help text and documentation

#### 5.2 Command Implementation
- Implement handlers for each command
- Connect CLI commands to the core components
- Add progress indicators and error reporting

#### Pseudocode:
```rust
fn build_cli() -> Command {
    Command::new("ragcli")
        .about("CLI tool for website crawling and RAG indexing")
        .subcommand_required(true)
        .subcommand(
            Command::new("crawl")
                .about("Crawl a website")
                .arg(arg!(<URL> "URL to crawl"))
                .arg(
                    arg!(-d --depth <DEPTH> "Crawl depth")
                        .value_parser(value_parser!(u32))
                        .default_value("2")
                )
                .arg(
                    arg!(-r --rate <RATE> "Rate limit in milliseconds")
                        .value_parser```rust
                .arg(
                    arg!(-r --rate <RATE> "Rate limit in milliseconds")
                        .value_parser(value_parser!(u64))
                        .default_value("1000")
                )
                .arg(
                    arg!(-o --output <FILE> "Save crawled content to file")
                        .required(false)
                )
                .arg(
                    arg!(-e --exclude <SELECTORS> "CSS selectors to exclude (comma-separated)")
                        .default_value("nav,footer,header,.ads,#comments")
                )
                .arg(
                    arg!(-i --include <SELECTORS> "CSS selectors to include (comma-separated)")
                        .required(false)
                )
        )
        .subcommand(
            Command::new("index")
                .about("Index crawled content")
                .arg(arg!(<SOURCE> "Source to index (URL or file)"))
                .arg(
                    arg!(-c --"chunk-size" <SIZE> "Chunk size in characters")
                        .value_parser(value_parser!(usize))
                        .default_value("1000")
                )
                .arg(
                    arg!(-m --model <MODEL> "LLM model for summaries")
                        .default_value("default-small")
                )
                .arg(
                    arg!(-f --force "Force reindex")
                        .action(ArgAction::SetTrue)
                )
        )
        .subcommand(
            Command::new("search")
                .about("Search the index")
                .arg(arg!(<QUERY> "Search query"))
                .arg(
                    arg!(-s --source <SOURCE> "Filter by source domain")
                        .required(false)
                )
                .arg(
                    arg!(-l --limit <LIMIT> "Limit results")
                        .value_parser(value_parser!(usize))
                        .default_value("10")
                )
                .arg(
                    arg!(-f --format <FORMAT> "Output format (text|json)")
                        .value_parser(["text", "json"])
                        .default_value("text")
                )
        )
        .subcommand(
            Command::new("list")
                .about("List indexed websites")
                .arg(
                    arg!(-d --details "Show detailed information")
                        .action(ArgAction::SetTrue)
                )
        )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = build_cli().get_matches();
    
    // Database setup
    let db = Database::new("index.db").await?;
    
    match matches.subcommand() {
        Some(("crawl", sub_matches)) => {
            let url = sub_matches.get_one::<String>("URL").unwrap();
            let depth = *sub_matches.get_one::<u32>("depth").unwrap();
            let rate = *sub_matches.get_one::<u64>("rate").unwrap();
            let exclude = sub_matches.get_one::<String>("exclude")
                .map(|s| s.split(',').map(String::from).collect::<Vec<_>>())
                .unwrap_or_default();
            let include = sub_matches.get_one::<String>("include")
                .map(|s| s.split(',').map(String::from).collect::<Vec<_>>())
                .unwrap_or_default();
            
            let config = CrawlerConfig {
                max_depth: depth,
                max_pages: 1000,
                rate_limit_ms: rate,
                respect_robots_txt: true,
                user_agent: "ragcli/0.1".to_string(),
                content_selectors: include,
                exclude_selectors: exclude,
            };
            
            println!("Crawling {}...", url);
            let pages = crawl_website(url, config).await?;
            println!("Crawled {} pages", pages.len());
            
            if let Some(output_file) = sub_matches.get_one::<String>("output") {
                save_to_file(&pages, output_file).await?;
                println!("Saved crawled content to {}", output_file);
            }
        },
        Some(("index", sub_matches)) => {
            let source = sub_matches.get_one::<String>("SOURCE").unwrap();
            let chunk_size = *sub_matches.get_one::<usize>("chunk-size").unwrap();
            let model = sub_matches.get_one::<String>("model").unwrap();
            let force = sub_matches.get_flag("force");
            
            let pages = if source.starts_with("http") {
                println!("Crawling {}...", source);
                let config = CrawlerConfig {
                    max_depth: 2,
                    max_pages: 1000,
                    rate_limit_ms: 1000,
                    respect_robots_txt: true,
                    user_agent: "ragcli/0.1".to_string(),
                    content_selectors: vec![],
                    exclude_selectors: vec![
                        "nav".to_string(), "footer".to_string(), "header".to_string(),
                        ".ads".to_string(), "#comments".to_string()
                    ],
                };
                crawl_website(source, config).await?
            } else {
                println!("Loading from file {}...", source);
                load_from_file(source).await?
            };
            
            println!("Processing {} pages...", pages.len());
            
            let processor_options = ProcessorOptions {
                chunk_options: ChunkOptions {
                    target_chunk_size: chunk_size,
                    overlap_size: chunk_size / 10,
                },
                llm_model: model.clone(),
                embedding_dimensions: 384,
            };
            
            let mut total_chunks = 0;
            for page in pages {
                let chunks = process_content(page.clone(), processor_options.clone()).await?;
                total_chunks += chunks.len();
                
                println!("Indexing {} chunks from {}...", chunks.len(), page.url);
                db.update_website_index(&page.url, chunks).await?;
            }
            
            println!("Indexed {} chunks across {} pages", total_chunks, pages.len());
        },
        Some(("search", sub_matches)) => {
            let query = sub_matches.get_one::<String>("QUERY").unwrap();
            let limit = *sub_matches.get_one::<usize>("limit").unwrap();
            let source = sub_matches.get_one::<String>("source").map(String::from);
            let format = sub_matches.get_one::<String>("format").unwrap();
            
            let options = SearchOptions {
                limit,
                min_score: 0.7,
                source_filter: source,
                date_range: None,
            };
            
            println!("Searching for: {}", query);
            let results = search_index(&db, query, options).await?;
            
            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&results)?);
                },
                _ => {
                    println!("Found {} results", results.len());
                    for (i, result) in results.iter().enumerate() {
                        println!("{}. {} (Score: {:.2})", i + 1, result.summary, result.score);
                        println!("   URL: {}", result.url);
                        println!("   Context: {}", result.context);
                        println!();
                    }
                }
            }
        },
        Some(("list", sub_matches)) => {
            let details = sub_matches.get_flag("details");
            
            let websites = db.list_websites().await?;
            
            println!("Indexed websites: {}", websites.len());
            for website in websites {
                if details {
                    println!("URL: {}", website.url);
                    println!("Domain: {}", website.domain);
                    println!("First indexed: {}", format_timestamp(website.first_index_date));
                    println!("Last indexed: {}", format_timestamp(website.last_index_date));
                    println!("Pages: {}", website.page_count);
                    println!("Status: {}", website.status);
                    println!();
                } else {
                    println!("{} - {} pages (Last indexed: {})", 
                        website.domain, 
                        website.page_count,
                        format_timestamp(website.last_index_date));
                }
            }
        },
        _ => unreachable!(),
    }
    
    Ok(())
}
```

#### Acceptance Criteria:
- CLI commands are intuitive and well-documented
- Commands provide appropriate feedback and error messages
- Progress indicators are included for long-running operations
- Search results are formatted for readability
- Management commands provide useful information

## Edge Cases and Error Handling

### 1. Crawling Edge Cases
- **Handling robots.txt**: Respect robots.txt directives and implement exponential backoff for rate limiting
- **Infinite loops**: Detect and break cycles in website navigation
- **Malformed HTML**: Handle malformed HTML gracefully without crashing
- **Content extraction**: Provide fallback methods when content selectors fail
- **Timeouts**: Implement timeout handling for slow websites or network issues
- **Security**: Avoid crawling password-protected or sensitive areas

### 2. Processing Edge Cases
- **Multilingual content**: Detect and handle content in different languages
- **Extremely large content**: Handle content that exceeds LLM context windows by splitting appropriately
- **Empty or low-quality content**: Detect and skip pages with insufficient content
- **Markdown conversion issues**: Handle conversion errors with fallback to plaintext
- **API rate limits**: Implement backoff strategies for LLM API rate limits

### 3. Index Edge Cases
- **Duplicate content**: Detect and handle duplicate content across different URLs
- **Index size limitations**: Implement pruning strategies for large indices
- **Database concurrency**: Handle concurrent database access safely
- **Database corruption**: Implement backup and recovery mechanisms
- **Incomplete indexing**: Provide recovery mechanisms for partially completed indexing

### 4. Search Edge Cases
- **Zero results**: Handle queries with no matching results gracefully
- **Very common queries**: Optimize for queries that return many results
- **Malformed queries**: Sanitize and validate user input
- **Relevance scoring**: Implement fallback strategy when embeddings are not effective
- **Performance degradation**: Monitor and optimize for index size growth

## Dependencies and Prerequisites

### 1. Core Dependencies
- **Tokio**: Async runtime for Rust
- **Clap**: Command-line argument parsing
- **Spider**: Web crawling library
- **LibSQL**: SQLite database with vector extensions
- **Scraper**: HTML parsing and content extraction
- **Markdown**: HTML to Markdown conversion
- **Reqwest**: HTTP client for API interactions
- **Serde**: Serialization and deserialization

### 2. External Services
- **Embedding API**: Service for generating embeddings
- **Small LLM API**: Service for generating summaries and context strings

### 3. System Requirements
- Sufficient disk space for storing the index
- Memory requirements for processing and generating embeddings
- Network access for crawling and API calls

## Implementation Timeline

1. Setup Project Structure (1 day)
2. Website Crawler Component (4 days)
   - Spider integration
   - Content cleaning and extraction
   - HTML to Markdown conversion
3. Content Processor Component (5 days)
   - Markdown chunking
   - Embedding generation
   - LLM integration
4. Index Manager Component (3 days)
   - LibSQL setup and schema design
   - CRUD operations implementation
5. Search System Component (3 days)
   - Vector search implementation
   - Result formatting and scoring
6. CLI Interface Component (2 days)
   - Command structure definition
   - Handler implementation
7. Testing and Refinement (3 days)
   - Unit and integration testing
   - Performance optimization
   - Error handling improvement

## Final Acceptance Criteria

The feature will be considered complete when:

1. Users can crawl websites with the following capabilities:
   - Configurable depth and rate limiting
   - Content extraction with removal of irrelevant elements
   - Clean Markdown conversion
   - Appropriate error handling

2. Content processing meets these requirements:
   - Effective chunking of Markdown content
   - Generation of high-quality embeddings
   - Production of useful summaries and context strings

3. Index management satisfies these criteria:
   - Efficient storage and retrieval of chunks and embeddings
   - Proper tracking of website metadata
   - Support for incremental updates

4. Search functionality provides:
   - Relevant results based on embedding similarity
   - Filtering by source website
   - Well-formatted output in multiple formats

5. CLI interface is:
   - Intuitive and well-documented
   - Provides appropriate feedback
   - Handles errors gracefully

6. The system as a whole:
   - Handles edge cases appropriately
   - Performs well with reasonably sized indices
   - Is maintainable and extensible

## Tech Lead Enhancements to the Architect's Plan

Here are additional sections providing context, specific file modifications, API references, implementation patterns, and integration points, as requested.

### 12. Specific Files and Functions to be Modified/Created

This feature will primarily involve creating new modules and files, but some existing ones will be modified.  Here's a breakdown:

**New Files/Modules (under `src/`):**

*   `src/crawler/`:  This new module will house all crawling logic.
    *   `src/crawler/mod.rs`:  Module definition.
    *   `src/crawler/spider_integration.rs`:  Integration with the `spider` crate.  Contains `crawl_website` function (from pseudocode).
    *   `src/crawler/config.rs`:  `CrawlerConfig` struct and builder (from pseudocode).
    *   `src/crawler/error.rs`:  `CrawlError` enum.  Extends existing `src/error.rs`.
    *   `src/crawler/content_extraction.rs`: Functions like `clean_html` and `extract_metadata` (from pseudocode), utilizing `scraper`.
*   `src/processor/`: This new module will handle content processing.
    *   `src/processor/mod.rs`: Module definition.
    *   `src/processor/chunking.rs`: `chunk_markdown` function (from pseudocode).
    *   `src/processor/embedding.rs`: `generate_embedding` function (from pseudocode). This will likely involve interacting with `src/gemini/models.rs`, specifically `embed_content`.
    *   `src/processor/llm_integration.rs`:  `generate_summary` and `generate_context_string` (from pseudocode).  This will interact with `src/gemini/models.rs`, using `generate_content`.
    *   `src/processor/config.rs`: `ProcessorOptions` and `ChunkOptions` structs (from pseudocode).
    *   `src/processor/error.rs`:  `ProcessError` enum. Extends existing `src/error.rs`.
*   `src/index/`: This new module will handle database interactions.
    *   `src/index/mod.rs`: Module definition.
    *   `src/index/database.rs`: `Database` struct and methods (from pseudocode), including `new`, `run_migrations`, `update_website_index`, etc.
    *   `src/index/schema.rs`:  Defines the SQL schema and migration logic.
    *   `src/index/error.rs`: `DbError` enum. Extends existing `src/error.rs`.
*   `src/search/`: This new module will handle search queries.
    *   `src/search/mod.rs`: Module definition
    *   `src/search/search.rs`: `search_index` function (from pseudocode), including `SearchOptions` struct.
    *   `src/search/error.rs`: `SearchError` enum. Extends existing `src/error.rs`.

**Modified Files:**

*   `src/main.rs`:  Clap CLI argument parsing and command dispatch (as in pseudocode).  This will need to instantiate the `Client` and the new `Database`, `Crawler`, `Processor`, and `Search` components, passing them to the appropriate command handlers.
*   `src/lib.rs`:  May need to expose new modules publicly.
*   `src/error.rs`:  Will be extended with new error enums from the `crawler`, `processor`, `index`, and `search` modules.
*  `src/gemini/models.rs`: Potentially modified if existing embedding functions don't match the needs of this feature. Add helper methods for embedding and content generation, if necessary.
* `Cargo.toml`: Add new dependencies: `spider`, `scraper`, `libsql`, `futures`, `clap`, `thiserror`.

### 13. API References and Documentation Links

*   **Spider:** [https://docs.rs/spider/latest/spider/](https://docs.rs/spider/latest/spider/)
*   **Scraper:** [https://docs.rs/scraper/latest/scraper/](https://docs.rs/scraper/latest/scraper/)
*   **LibSQL:** [https://docs.rs/libsql/latest/libsql/](https://docs.rs/libsql/latest/libsql/)  (Note: Use the `libsql` crate, *not* `libsqlite3-sys`.  LibSQL is the Turso fork of SQLite and provides better async support.)  Also, refer to the Turso/LibSQL documentation: [https://libsql.org/](https://libsql.org/) and [https://github.com/libsql/libsql](https://github.com/libsql/libsql) for vector search capabilities.  This project will specifically need the `vss` extension.
*   **Reqwest:** [https://docs.rs/reqwest/latest/reqwest/](https://docs.rs/reqwest/latest/reqwest/) (Already a dependency)
*   **Serde:** [https://docs.rs/serde/latest/serde/](https://docs.rs/serde/latest/serde/) (Already a dependency)
*   **Tokio:** [https://docs.rs/tokio/latest/tokio/](https://docs.rs/tokio/latest/tokio/) (Already a dependency)
*   **Clap:** [https://docs.rs/clap/latest/clap/](https://docs.rs/clap/latest/clap/)
*   **thiserror:** [https://docs.rs/thiserror/latest/thiserror/](https://docs.rs/thiserror/latest/thiserror/) (Already a dependency)
*   **pulldown-cmark:** [https://docs.rs/pulldown-cmark/latest/pulldown_cmark/](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/)
*    **Gemini API:** [https://ai.google.dev/docs](https://ai.google.dev/docs)

### 14. Context within Existing Architecture

This feature significantly expands the `hal` crate.  Currently, it focuses on direct interaction with the Gemini API.  This feature adds a substantial "local" component, dealing with web crawling, data storage, and local search.

*   **Relationship to `src/gemini/`:**  The `processor` component will leverage the existing `src/gemini/` code for embedding generation and LLM interaction (summaries, context). The `crawler` component will be independent of `src/gemini/`.
*   **New Top-Level Concerns:**  Crawling, indexing, and searching become major functionalities, alongside the existing Gemini API client.
*   **Data Flow:** The general data flow will be:  `crawler` -> `processor` -> `index` -> `search`. The `src/main.rs` CLI will orchestrate this flow based on user commands.

### 15. Implementation Patterns

*   **Builder Pattern:** For `CrawlerConfig` and other configuration structs, the builder pattern provides a flexible and readable way to set options.
*   **Error Handling:** Use the `thiserror` crate to define custom error enums (`CrawlError`, `ProcessError`, `DbError`, `SearchError`) within each module.  These should all implement `From` for the existing `crate::error::Error`, allowing easy propagation with the `?` operator.
*   **Asynchronous Operations:**  Use `async`/`await` throughout, leveraging Tokio for concurrency.  Use `tokio::spawn` for tasks that can run in the background (e.g., embedding generation, LLM calls).  Use `futures::future::join_all` for awaiting multiple tasks.
*   **Bounded Concurrency:**  For external API calls (embedding, LLM), use a `tokio::sync::Semaphore` to limit the number of concurrent requests, preventing rate-limiting issues.
*   **Transactions:** Use database transactions (as shown in the `Database` pseudocode) to ensure data consistency when updating the index.
*   **Dependency Injection:** Pass dependencies (e.g., `HttpClient`, `Database` connection) to components rather than creating them internally. This improves testability.
*    **Modularity**: Utilize modules, as shown in the file section, to encapsulate different logic and keep the code maintainable.

### 16. Integration Points

*   **Gemini API:** The `processor` component integrates with the Gemini API (via the existing `src/gemini/` code) for:
    *   **Embedding Generation:**  Use `src/gemini/models::embed_content` (or a similar function) to generate embeddings for text chunks.
    *   **Summary and Context Generation:**  Use `src/gemini/models::generate_content` to generate summaries and context strings.
*   **LibSQL:** The `index` component integrates with LibSQL for:
    *   **Data Storage:** Storing website metadata, chunks, and embeddings.
    *   **Vector Search:** Using LibSQL's vector search capabilities for similarity search.
*   **CLI (`src/main.rs`):** The CLI integrates all components, orchestrating the workflow based on user input.
*   **Existing TUI:** While not directly modified by this feature, the new `search` functionality could *eventually* be integrated into the existing TUI (`src/tui/`) to provide a search interface within the chat application. This is beyond the scope of the initial implementation, but is a future possibility.
*  **`src/error.rs`**: New error types defined in modules are added to the main `Error` enum.

### Additional Considerations and Recommendations

*   **Testing:**  Thorough unit and integration tests are crucial.  Use `#[cfg(test)]` to create test modules within each new file. Mock external dependencies (Gemini API, LibSQL in some cases) for unit tests.
*   **Logging:**  Continue using the `tracing` crate for logging.  Add informative logs at different levels (debug, info, warn, error) to track the progress of crawling, processing, indexing, and searching.
*   **Configuration:** Consider using a configuration file (e.g., TOML) for crawler settings, LLM model selection, and other parameters.
*   **Rate Limiting:** Implement robust rate limiting and exponential backoff for all external API calls (Gemini, and potentially embedding API if it's separate).
*   **User Agent:** Set a descriptive User-Agent header for all HTTP requests (both crawling and API calls).
*   **Robots.txt:** Ensure the crawler respects `robots.txt` rules. The `spider` crate likely handles this, but double-check.
*   **Incremental Crawling/Indexing:** Design the system to support incremental crawling and indexing.  The `last_index_date` in the database schema is a good start.  Consider how to detect and handle changed content on subsequent crawls.
*   **Documentation**: Add comprehensive documentation to all new modules, structs, functions.

This enhanced plan provides a much more detailed roadmap for implementing the website crawling, indexing, and search feature. It clarifies the technical details, addresses potential issues, and integrates the new functionality within the existing codebase structure. The use of specific crates and patterns is defined, and the interactions between components are clearly outlined.
