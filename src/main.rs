mod tui;

use clap::{Args, Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use tokio::sync::mpsc;

#[derive(Parser)]
#[command(author, version, about = "A Rust client for Google's Gemini AI API", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start an interactive chat session with Gemini
    Chat(ChatArgs),

    /// Crawl a website and save the content
    Crawl(CrawlArgs),

    /// Index crawled content for RAG
    Index(IndexArgs),

    /// Search the indexed content
    Search(SearchArgs),

    /// List indexed websites
    List(ListArgs),

    /// Reembed all chunks in the index with new embeddings
    Reembed(ReembedArgs),
}

#[derive(Args)]
struct ChatArgs {
    /// Gemini model to use (default: gemini-2.0-flash)
    #[arg(short, long, default_value = "gemini-2.0-flash")]
    model: String,
}

#[derive(Args)]
struct CrawlArgs {
    /// URL to crawl
    #[arg(required = true)]
    url: String,

    /// Crawl depth
    #[arg(short, long, default_value = "2")]
    depth: u32,

    /// Rate limit in milliseconds
    #[arg(short, long, default_value = "1000")]
    rate: u64,

    /// Save crawled content to file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// CSS selectors to exclude (comma-separated)
    #[arg(short, long, default_value = "nav,footer,header,.ads,#comments")]
    exclude: String,

    /// CSS selectors to include (comma-separated)
    #[arg(short, long)]
    include: Option<String>,

    /// Maximum number of pages to crawl
    #[arg(short = 'p', long, default_value = "100")]
    max_pages: u32,
}

#[derive(Args)]
struct IndexArgs {
    /// Source to index (URL or file)
    #[arg(required = true)]
    source: String,

    /// Chunk size in characters
    #[arg(short, long, default_value = "1000")]
    chunk_size: usize,

    /// LLM model for summaries
    #[arg(short, long, default_value = "gemini-2.0-flash-lite")]
    model: String,

    /// Force reindex
    #[arg(short, long)]
    force: bool,

    /// Database path
    #[arg(short, long, default_value = "index.db")]
    database: PathBuf,
}

#[derive(Args)]
struct SearchArgs {
    /// Search query
    #[arg(required = true)]
    query: String,

    /// Filter by source domain
    #[arg(short, long)]
    source: Option<String>,

    /// Limit results
    #[arg(short, long, default_value = "5")]
    limit: usize,

    /// Output format (text|json)
    #[arg(short, long, default_value = "text", value_parser = ["text", "json"])]
    format: String,

    /// Database path
    #[arg(short, long, default_value = "index.db")]
    database: PathBuf,

    /// Use vector search only (no LLM)
    #[arg(short = 'v', long, default_value = "false")]
    vector_search_only: bool,

    /// LLM model to use for RAG
    #[arg(short = 'm', long, default_value = "gemini-2.0-flash")]
    model: String,
}

#[derive(Args)]
struct ListArgs {
    /// Show detailed information
    #[arg(short, long)]
    details: bool,

    /// Database path
    #[arg(short, long, default_value = "index.db")]
    database: PathBuf,
}

#[derive(Args)]
struct ReembedArgs {
    /// Database path
    #[arg(short, long, default_value = "index.db")]
    database: PathBuf,

    /// Number of concurrent embedding operations
    #[arg(short, long, default_value = "5")]
    concurrency: usize,

    /// Filter by source domain
    #[arg(short, long)]
    source: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Execute the appropriate command
    match cli.command {
        Some(Commands::Chat(args)) => {
            // Get API key from environment variable
            let api_key = std::env::var("GEMINI_API_KEY")
                .expect("GEMINI_API_KEY environment variable must be set");

            // Print the selected model
            println!("Starting chat with model: {}", args.model);

            // Run the TUI application
            tui::run(api_key, args.model).await?;
        }
        Some(Commands::Crawl(args)) => {
            crawl_command(args).await?;
        }
        Some(Commands::Index(args)) => {
            index_command(args).await?;
        }
        Some(Commands::Search(args)) => {
            search_command(args).await?;
        }
        Some(Commands::List(args)) => {
            list_command(args).await?;
        }
        Some(Commands::Reembed(args)) => {
            reembed_command(args).await?;
        }
        None => {
            // If no command is provided, show help
            let _ = Cli::parse_from(["--help"]);
        }
    }

    Ok(())
}

async fn crawl_command(args: CrawlArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!("Crawling {}...", args.url);

    // Create crawler configuration
    let config = hal::crawler::CrawlerConfig::builder()
        .max_depth(args.depth)
        .max_pages(args.max_pages)
        .rate_limit_ms(args.rate)
        .respect_robots_txt(true)
        .user_agent("hal-rag/0.1".to_string())
        .exclude_selectors(args.exclude.split(',').map(String::from).collect())
        .content_selectors(
            args.include
                .map(|s| s.split(',').map(String::from).collect())
                .unwrap_or_default(),
        )
        .build();

    // Crawl the website
    let pages = hal::crawler::crawl_website(&args.url, config).await?;

    println!("Crawled {} pages", pages.len());

    // Save to file if output is specified
    if let Some(output_file) = args.output {
        // Serialize pages to JSON
        let json = serde_json::to_string_pretty(&pages)?;
        tokio::fs::write(output_file.clone(), json).await?;
        println!("Saved crawled content to {}", output_file.display());
    }

    Ok(())
}

async fn index_command(args: IndexArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment variable
    let api_key =
        std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable must be set");

    // Create client
    let client = hal::Client::with_api_key_rate_limited(api_key);

    // Create database connection
    let db = hal::index::Database::new_from_path(&args.database.to_string_lossy()).await?;

    let pages = if args.source.starts_with("http") {
        println!("Crawling {}...", args.source);

        // Create crawler configuration
        let config = hal::crawler::CrawlerConfig::builder()
            .max_depth(2)
            .max_pages(100)
            .rate_limit_ms(1000)
            .respect_robots_txt(true)
            .user_agent("hal-rag/0.1".to_string())
            .exclude_selectors(vec![
                "nav".to_string(),
                "footer".to_string(),
                "header".to_string(),
                ".ads".to_string(),
                "#comments".to_string(),
            ])
            .build();

        // Crawl the website
        hal::crawler::crawl_website(&args.source, config).await?
    } else {
        println!("Loading from file {}...", args.source);

        // Read file
        let content = tokio::fs::read_to_string(&args.source).await?;

        // Parse JSON
        serde_json::from_str(&content)?
    };

    println!("Processing {} pages...", pages.len());

    // Create processor options
    let processor_config = hal::processor::ProcessorConfig::builder()
        .chunk_options(hal::processor::ChunkOptions {
            target_chunk_size: args.chunk_size,
            overlap_size: args.chunk_size / 10,
        })
        .llm_model(args.model.clone())
        .embedding_dimensions(768)
        .build();

    // Process and index pages
    let mut total_chunks = 0;
    let mut indexed_pages = 0;

    // Track unique base URLs to count pages per website
    use std::collections::HashMap;
    use url::Url;

    let mut website_pages = HashMap::new();

    // Group pages by base URL
    for page in &pages {
        if let Ok(parsed_url) = Url::parse(&page.url) {
            if let Some(host) = parsed_url.host_str() {
                let base_url = format!("{}://{}", parsed_url.scheme(), host);
                website_pages
                    .entry(base_url)
                    .or_insert_with(Vec::new)
                    .push(page);
            }
        }
    }

    // Get the number of websites before consuming the HashMap
    let website_count = website_pages.len();

    // Process and index pages by website
    for (base_url, site_pages) in website_pages {
        println!(
            "Processing website: {} ({} pages)",
            base_url,
            site_pages.len()
        );

        for page in site_pages {
            // Process content
            let chunks =
                hal::processor::process_content(&client, page.clone(), processor_config.clone())
                    .await?;
            total_chunks += chunks.len();
            indexed_pages += 1;

            println!("Indexing {} chunks from {}...", chunks.len(), page.url);

            // Update website index
            db.update_website_index(&page.url, chunks).await?;
        }
    }

    println!(
        "Indexed {} chunks across {} pages from {} websites",
        total_chunks, indexed_pages, website_count
    );

    Ok(())
}

async fn search_command(args: SearchArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Create database connection
    let db = hal::index::Database::new_from_path(&args.database.to_string_lossy()).await?;

    println!("Searching for: {}", args.query);

    // Create a client
    let api_key =
        std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable must be set");
    let client = hal::Client::with_api_key_rate_limited(api_key);

    // Create search options
    let options = hal::search::SearchOptions {
        limit: args.limit,
        source_filter: args.source,
        date_range: None,
    };

    // Search the index
    let results = hal::search::search_index_with_client(&db, &client, &args.query, options).await?;

    // If vector search only, output results directly
    if args.vector_search_only {
        // Output results
        match args.format.as_str() {
            "json" => {
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
            _ => {
                println!("Found {} results", results.len());
                for (i, result) in results.iter().enumerate() {
                    println!("{}. {}", i + 1, result.text);
                    println!("   URL: {}", result.url);
                    println!("   Context: {}", result.context);
                    println!();
                }
            }
        }
    } else {
        // Use RAG to generate an answer
        println!("Generating answer using RAG...");

        // Prepare context from search results
        let context = prepare_rag_context(&results);

        // Generate answer using LLM
        let answer = generate_answer_with_rag(&client, &args.query, &context, &args.model).await?;

        // Output results
        match args.format.as_str() {
            "json" => {
                let json_response = serde_json::json!({
                    "query": args.query,
                    "answer": answer,
                    "sources": results.iter().map(|r| {
                        serde_json::json!({
                            "text": r.text,
                            "url": r.url,
                            "context": r.context
                        })
                    }).collect::<Vec<_>>()
                });
                println!("{}", serde_json::to_string_pretty(&json_response)?);
            }
            _ => {
                println!("\nAnswer:");
                println!("{}", answer);
                println!("\nSources:");
                for (i, result) in results.iter().enumerate() {
                    println!("{}. {}", i + 1, result.url);
                }
                println!();
            }
        }
    }

    Ok(())
}

/// Prepare context from search results for RAG
fn prepare_rag_context(results: &[hal::search::SearchResult]) -> String {
    let mut context = String::new();

    for (i, result) in results.iter().enumerate() {
        context.push_str(&format!("Source {}:\n", i + 1));
        context.push_str(&format!("URL: {}\n", result.url));
        context.push_str(&format!("Content: {}\n\n", result.text));
    }

    context
}

/// Generate an answer using RAG
async fn generate_answer_with_rag(
    client: &hal::Client,
    query: &str,
    context: &str,
    model: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    use hal::prelude::Content;

    // Create system prompt for RAG
    let system_prompt = "You are a helpful assistant that answers questions based on the provided context. \
    Use only the information from the context to answer the question. \
    If the context doesn't contain enough information to answer the question fully, \
    acknowledge the limitations and provide the best answer possible with the available information. \
    Be concise and accurate.";

    // Create user prompt with context and query
    let user_prompt = format!("Context:\n{}\n\nQuestion: {}\n\nAnswer:", context, query);

    // Create content for the request
    let system_content = Content::new().with_role("system").with_text(system_prompt);
    let user_content = Content::new().with_role("user").with_text(user_prompt);

    // Generate content
    let response = client
        .models()
        .generate_content(model, Some(system_content), vec![user_content])
        .await
        .map_err(|e| format!("Failed to generate answer: {}", e))?;

    // Return the generated text
    Ok(response.text())
}

async fn list_command(args: ListArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Create database connection
    let db = hal::index::Database::new_from_path(&args.database.to_string_lossy()).await?;

    // List websites
    let websites = db.list_websites().await?;

    println!("Indexed websites: {}", websites.len());

    // Format timestamp function
    let format_timestamp = |ts: i64| -> String {
        use chrono::{DateTime, TimeZone, Utc};
        let dt: DateTime<Utc> = Utc.timestamp_opt(ts, 0).unwrap();
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    };

    // Display websites
    for website in websites {
        if args.details {
            println!("URL: {}", website.url);
            println!("Domain: {}", website.domain);
            println!(
                "First indexed: {}",
                format_timestamp(website.first_index_date)
            );
            println!(
                "Last indexed: {}",
                format_timestamp(website.last_index_date)
            );
            println!("Pages: {}", website.page_count);
            println!("Status: {}", website.status);
            println!();
        } else {
            println!(
                "{} - {} pages (Last indexed: {})",
                website.domain,
                website.page_count,
                format_timestamp(website.last_index_date)
            );
        }
    }

    Ok(())
}

async fn reembed_command(args: ReembedArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Create database connection
    let db = hal::index::Database::new_from_path(&args.database.to_string_lossy()).await?;

    println!("Reembedding all chunks in the index with new embeddings...");

    // Display source filter if specified
    if let Some(source) = &args.source {
        println!("Filtering by source domain: {}", source);
    }

    println!("Using concurrency level: {}", args.concurrency);

    // Get API key from environment variable
    let api_key =
        std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable must be set");

    // Create client for embedding generation
    let client = hal::Client::with_api_key_rate_limited(api_key);

    // Create a channel for progress updates
    let (progress_sender, mut progress_receiver) = mpsc::channel(100);

    // First, count the total number of chunks to process
    let total_chunks = count_chunks_to_reembed(&db, args.source.clone()).await?;

    // Create progress bar
    let progress_bar = ProgressBar::new(total_chunks as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("##-"),
    );
    progress_bar.set_message("Reembedding chunks...");

    // Start timer
    let start_time = std::time::Instant::now();

    // Spawn a task to process progress updates
    let progress_handle = tokio::spawn({
        let progress_bar = progress_bar.clone();
        async move {
            while let Some((chunk_id, url)) = progress_receiver.recv().await {
                progress_bar.inc(1);
                // Only update the message, don't print a new line
                progress_bar.set_message(format!("Processed chunk {} from {}", chunk_id, url));
            }
            // Signal that we're done processing updates
            progress_bar.finish_with_message("Reembedding completed");
        }
    });

    // Reembed all chunks in the index with new embeddings
    let reembedded_count = db
        .reembed_all_chunks(
            &client,
            args.concurrency,
            args.source.clone(),
            Some(progress_sender),
        )
        .await?;

    // Wait for progress task to complete (it will end when all senders are dropped)
    let _ = progress_handle.await;

    // Calculate elapsed time
    let elapsed = start_time.elapsed();

    println!("Reembedding completed successfully");
    println!("Reembedded {} chunks in {:.2?}", reembedded_count, elapsed);

    if reembedded_count > 0 {
        let avg_time = elapsed.as_millis() as f64 / reembedded_count as f64;
        println!("Average time per chunk: {:.2?}ms", avg_time);
    }

    Ok(())
}

/// Count the number of chunks that will be reembedded
async fn count_chunks_to_reembed(
    db: &hal::index::Database,
    source_filter: Option<String>,
) -> Result<usize, Box<dyn std::error::Error>> {
    // Build the SQL query
    let mut sql =
        String::from("SELECT COUNT(*) FROM chunks c JOIN websites w ON c.website_id = w.id");

    // Add source filter if specified
    let mut params: Vec<libsql::Value> = Vec::new();
    if let Some(source) = &source_filter {
        sql.push_str(" WHERE w.domain LIKE ?");
        params.push(format!("%{}%", source).into());
    }

    // Execute query
    let mut rows = db.execute_query(&sql, params).await?;

    // Get the count
    let row = match rows.next().await {
        Ok(Some(row)) => row,
        Ok(None) => return Ok(0),
        Err(e) => return Err(format!("Failed to get count: {}", e).into()),
    };

    let count: i64 = match row.get(0) {
        Ok(count) => count,
        Err(e) => return Err(format!("Failed to get count from row: {}", e).into()),
    };

    Ok(count as usize)
}
