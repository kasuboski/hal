use quick_xml::{de::from_str, se::to_string};
use serde::{Deserialize, Serialize};
use std::{io, path::Path, path::PathBuf};
use tokio::fs;
use url::Url;

use super::CrawledPage;
use super::PageMetadata;

/// Storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Base path for storage
    pub base_path: PathBuf,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            base_path: PathBuf::from(".hal/crawler"),
        }
    }
}

/// XML representation of pages for storage
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename = "pages")]
pub struct Pages {
    #[serde(rename = "page")]
    pub pages: Vec<PageEntry>,
}

/// XML representation of a single page for storage
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PageEntry {
    /// URL of the page
    pub url: String,

    /// Content of the page in Markdown format
    pub content: String,

    /// Metadata extracted from the page
    pub metadata: PageMetadata,
}

impl From<CrawledPage> for PageEntry {
    fn from(page: CrawledPage) -> Self {
        PageEntry {
            url: page.url,
            content: page.content,
            metadata: page.metadata,
        }
    }
}

impl From<PageEntry> for CrawledPage {
    fn from(entry: PageEntry) -> Self {
        CrawledPage {
            url: entry.url,
            content: entry.content,
            metadata: entry.metadata,
        }
    }
}

impl AsRef<PageEntry> for PageEntry {
    fn as_ref(&self) -> &PageEntry {
        self
    }
}

/// Error type for storage operations
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("XML error: {0}")]
    XMLError(#[from] quick_xml::Error),

    #[error("XML serialization error: {0}")]
    SerializeError(#[from] quick_xml::errors::serialize::SeError),

    #[error("XML deserialization error: {0}")]
    DeserializeError(#[from] quick_xml::errors::serialize::DeError),

    #[error("URL parsing error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("Invalid URL for storage: {0}")]
    InvalidUrl(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

type Result<T> = std::result::Result<T, StorageError>;

/// Storage manager for crawler pages
#[derive(Debug, Clone)]
pub struct Storage {
    config: StorageConfig,
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage {
    /// Create a new storage with default configuration
    pub fn new() -> Self {
        Self {
            config: StorageConfig::default(),
        }
    }

    /// Create a new storage with custom configuration
    pub fn with_config(config: StorageConfig) -> Self {
        Self { config }
    }

    /// Extracts the domain from a URL for use in storage path
    fn extract_domain(&self, url: &str) -> Result<String> {
        let parsed = Url::parse(url)?;
        let host = parsed
            .host_str()
            .ok_or_else(|| StorageError::InvalidUrl(url.to_string()))?;
        Ok(host.to_string())
    }

    /// Gets the storage path for a given URL
    fn get_storage_path(&self, url: &str) -> Result<std::path::PathBuf> {
        let domain = self.extract_domain(url)?;
        let parsed = Url::parse(url)?;

        // Create a filename from the URL path
        let path = parsed.path();
        let filename = if path.is_empty() || path == "/" {
            "index.xml".to_string()
        } else {
            // Replace non-alphanumeric characters with underscores
            let safe_path = path
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '/' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect::<String>();

            // Remove leading and trailing slashes
            let trimmed = safe_path.trim_matches('/');

            // Add .xml extension
            format!("{}.xml", trimmed.replace('/', "_"))
        };

        Ok(self.config.base_path.join(domain).join(filename))
    }

    /// Creates a URL from a domain and filename
    /// 
    /// This is the reverse of get_storage_path - it takes a filepath and converts it back to a URL
    fn create_url_from_filename(&self, domain: &str, filename: &str) -> String {
        if filename == "index.xml" {
            format!("https://{}", domain)
        } else {
            // Remove .xml extension
            let path_part = filename.trim_end_matches(".xml");
            // Convert underscores back to slashes if needed
            let path_part = path_part.replace('_', "/");
            format!("https://{}/{}", domain, path_part)
        }
    }

    /// Creates necessary directories for storage
    async fn ensure_directories(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        Ok(())
    }
    
    /// Stores a single page entry to XML file
    pub async fn store(&self, entry: &PageEntry) -> Result<()> {
        let storage_path = self.get_storage_path(&entry.url)?;
        self.ensure_directories(&storage_path).await?;

        // Wrap in Pages struct for XML structure
        let pages = Pages {
            pages: vec![entry.clone()],
        };
        let xml = to_string(&pages)?;

        fs::write(
            storage_path,
            format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", xml),
        )
        .await?;
        Ok(())
    }

    /// Stores multiple page entries in their respective XML files
    ///
    /// Accepts any type that can be converted into an iterator of PageEntry references.
    /// This allows it to work with vectors, arrays, slices, and other collections.
    pub async fn store_batch<I>(&self, entries: I) -> Result<()>
    where
        I: IntoIterator,
        I::Item: AsRef<PageEntry>,
    {
        for entry in entries {
            self.store(entry.as_ref()).await?;
        }
        Ok(())
    }

    /// Loads a page entry from an XML file
    pub async fn load(&self, url: &str) -> Result<PageEntry> {
        let storage_path = self.get_storage_path(url)?;
        let xml_content = fs::read_to_string(storage_path).await?;
        let pages: Pages = from_str(&xml_content)?;

        // Since we store one page per file, take the first one
        pages.pages.into_iter().next().ok_or_else(|| {
            StorageError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "XML file contains no pages",
            ))
        })
    }

    /// Loads all pages for a given domain
    /// 
    /// The domain parameter can be either a domain name (e.g., "example.com") or a full URL.
    /// If a URL is provided, the domain will be extracted from it.
    pub async fn load_domain(&self, domain_or_url: &str) -> Result<Vec<PageEntry>> {
        // Try to extract domain if a URL was passed
        let domain = if domain_or_url.contains("://") {
            self.extract_domain(domain_or_url)?
        } else {
            domain_or_url.to_string()
        };
        
        let base_path = self.config.base_path.join(&domain);

        // Check if the domain directory exists
        if !fs::try_exists(&base_path).await? {
            return Err(StorageError::NotFound(format!(
                "No pages found for domain {}",
                domain
            )));
        }

        let mut entries = Vec::new();
        let mut dir_entries = fs::read_dir(base_path).await?;
        
        while let Some(entry) = dir_entries.next_entry().await? {
            let path = entry.path();
            
            if path.extension().is_some_and(|ext| ext == "xml") {
                // Create a URL from domain and filename for loading
                if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                    let url_path = self.create_url_from_filename(&domain, file_name);
                    
                    // Use the load method to load the page
                    match self.load(&url_path).await {
                        Ok(page) => entries.push(page),
                        Err(e) => {
                            eprintln!("Failed to load page {}: {}", url_path, e);
                            continue;
                        }
                    }
                }
            }
        }

        Ok(entries)
    }
}

// Create module-level functions that use the default storage for backward compatibility
/// Extracts the domain from a URL for use in storage path
pub fn extract_domain(url: &str) -> Result<String> {
    Storage::new().extract_domain(url)
}

/// Gets the storage path for a given URL
pub fn get_storage_path(url: &str) -> Result<std::path::PathBuf> {
    Storage::new().get_storage_path(url)
}

/// Stores a single page entry to XML file
pub async fn store(entry: &PageEntry) -> Result<()> {
    Storage::new().store(entry).await
}

/// Stores multiple page entries in their respective XML files
pub async fn store_batch<I>(entries: I) -> Result<()>
where
    I: IntoIterator,
    I::Item: AsRef<PageEntry>,
{
    Storage::new().store_batch(entries).await
}

/// Loads a page entry from an XML file
pub async fn load(url: &str) -> Result<PageEntry> {
    Storage::new().load(url).await
}

/// Loads all pages for a given domain
pub async fn load_domain(domain_or_url: &str) -> Result<Vec<PageEntry>> {
    Storage::new().load_domain(domain_or_url).await
}

/// Creates a URL from a domain and filename
/// 
/// This is the reverse of get_storage_path - it takes a filename and converts it back to a URL
pub fn create_url_from_filename(domain: &str, filename: &str) -> String {
    Storage::new().create_url_from_filename(domain, filename)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_domain_extraction_from_url() {
        let storage = Storage::new();
        assert_eq!(
            storage.extract_domain("https://example.com/page").unwrap(),
            "example.com"
        );
        
        // Test with module function
        assert_eq!(
            extract_domain("https://example.com/page").unwrap(),
            "example.com"
        );
        
        // Test with subdomain
        assert_eq!(
            extract_domain("http://blog.example.com/post").unwrap(),
            "blog.example.com"
        );
    }
    
    #[test]
    fn test_domain_extraction_error() {
        // Test with invalid URL
        let result = extract_domain("not-a-url");
        assert!(result.is_err());
        
        // More specifically, it should be a URL parsing error
        match result {
            Err(StorageError::UrlParse(_)) => (),
            _ => panic!("Expected UrlParse error"),
        }
    }

    #[test]
    fn test_get_storage_path() {
        // Test default storage path
        let storage = Storage::new();
        let path = storage.get_storage_path("https://example.com/").unwrap();
        assert_eq!(path, Path::new(".hal/crawler/example.com/index.xml"));

        // Test custom storage path
        let config = StorageConfig {
            base_path: PathBuf::from("/tmp/crawler"),
        };
        let storage = Storage::with_config(config);
        let path = storage.get_storage_path("https://example.com/").unwrap();
        assert_eq!(path, Path::new("/tmp/crawler/example.com/index.xml"));

        // Test with module-level function (using default path)
        let path = get_storage_path("https://example.com/page").unwrap();
        assert_eq!(path, Path::new(".hal/crawler/example.com/page.xml"));

        let path = get_storage_path("https://example.com/dir/page").unwrap();
        assert_eq!(path, Path::new(".hal/crawler/example.com/dir_page.xml"));
    }

    #[test]
    fn test_conversion() {
        let crawled = CrawledPage {
            url: "https://example.com/a".to_string(),
            content: "# Title\nContent".to_string(),
            metadata: PageMetadata {
                title: Some("Example Page".to_string()),
                description: Some("An example page".to_string()),
                domain: "https://example.com".to_string(),
                publication_date: None,
                author: None,
            },
        };

        let entry: PageEntry = crawled.clone().into();
        assert_eq!(entry.url, crawled.url);
        assert_eq!(entry.content, crawled.content);

        let back: CrawledPage = entry.into();
        assert_eq!(back.url, crawled.url);
        assert_eq!(back.content, crawled.content);
    }

    #[test]
    fn test_storage_with_custom_config() {
        let config = StorageConfig {
            base_path: PathBuf::from("/custom/path"),
        };
        let storage = Storage::with_config(config);

        assert_eq!(storage.config.base_path, PathBuf::from("/custom/path"));
    }
    
    #[test]
    fn test_create_url_from_filename() {
        let storage = Storage::new();
        
        // Test root URL
        assert_eq!(
            storage.create_url_from_filename("example.com", "index.xml"),
            "https://example.com"
        );
        
        // Test simple path
        assert_eq!(
            storage.create_url_from_filename("example.com", "page.xml"),
            "https://example.com/page"
        );
        
        // Test nested path
        assert_eq!(
            storage.create_url_from_filename("example.com", "dir_page.xml"),
            "https://example.com/dir/page"
        );
        
        // Test with module function
        assert_eq!(
            create_url_from_filename("example.com", "blog_post.xml"),
            "https://example.com/blog/post"
        );
    }
    
    #[tokio::test]
    async fn test_load_domain_with_url() {
        // Test load_domain with a URL instead of domain
        let config = StorageConfig {
            base_path: PathBuf::from("/tmp/test_load_domain"),
        };
        let storage = Storage::with_config(config);
        
        // This should extract the domain from the URL
        let result = storage.load_domain("https://example.com/some/page").await;
        
        // Since the path doesn't exist, it should return a NotFound error
        match result {
            Err(StorageError::NotFound(msg)) => {
                assert!(msg.contains("example.com"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }
}
