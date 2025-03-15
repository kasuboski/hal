//! Database operations for the index module

use crate::index::error::DbError;
use crate::index::schema;
use crate::index::{IndexedChunk, Website};
use libsql::{params, Connection, Row, Rows};
use std::time::{SystemTime, UNIX_EPOCH};

/// Database manager for the index
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Create a new database manager
    pub async fn new(conn: Connection) -> Result<Self, DbError> {
        // Initialize schema
        schema::initialize_schema(&conn).await?;

        Ok(Self { conn })
    }

    /// Create a new database manager from a path
    pub async fn new_from_path(path: &str) -> Result<Self, DbError> {
        let db = libsql::Builder::new_local(path)
            .build()
            .await
            .map_err(|e| DbError::Connection(format!("Failed to open database: {}", e)))?;

        let conn = db
            .connect()
            .map_err(|e| DbError::Connection(format!("Failed to connect to database: {}", e)))?;

        Self::new(conn).await
    }

    /// Execute a custom query with parameters
    pub async fn execute_query<P>(&self, sql: &str, params: P) -> Result<Rows, DbError>
    where
        P: libsql::params::IntoParams,
    {
        self.conn
            .query(sql, params)
            .await
            .map_err(|e| DbError::Query(format!("Failed to execute query: {}", e)))
    }

    /// Add a website to the index
    pub async fn add_website(&self, website: &Website) -> Result<i64, DbError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Parse the URL to extract the base URL
        let parsed_url = website
            .url
            .parse::<url::Url>()
            .map_err(|e| DbError::Data(format!("Failed to parse URL: {}", e)))?;

        // Extract the base URL (scheme + host)
        let base_url = match parsed_url.host_str() {
            Some(host) => format!("{}://{}", parsed_url.scheme(), host),
            None => return Err(DbError::Data("URL has no host".to_string())),
        };

        self.conn.execute(
            "INSERT INTO websites (url, domain, first_index_date, last_index_date, page_count, status) 
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(url) DO UPDATE SET
             domain = excluded.domain,
             last_index_date = ?,
             page_count = excluded.page_count,
             status = excluded.status",
            params![
                base_url,
                website.domain.clone(),
                website.first_index_date,
                website.last_index_date,
                website.page_count,
                website.status.clone(),
                now,
            ],
        ).await.map_err(|e| DbError::Query(format!("Failed to add website: {}", e)))?;

        // Get the ID of the inserted website using a query
        let mut rows = self
            .conn
            .query("SELECT last_insert_rowid()", params![])
            .await
            .map_err(|e| DbError::Query(format!("Failed to get last insert ID: {}", e)))?;

        // In libsql 0.6.0, next() is async and returns Result<Option<Row>>
        let row = match rows.next().await {
            Ok(Some(row)) => row,
            Ok(None) => {
                return Err(DbError::Data(
                    "No ID returned from last_insert_rowid()".to_string(),
                ))
            }
            Err(e) => return Err(DbError::Data(format!("Failed to get ID: {}", e))),
        };

        let id = row
            .get(0)
            .map_err(|e| DbError::Data(format!("Failed to get ID: {}", e)))?;
        Ok(id)
    }

    /// Get a website by URL
    pub async fn get_website_by_url(&self, url: &str) -> Result<Option<Website>, DbError> {
        // Parse the URL to extract the base URL
        let parsed_url = match url.parse::<url::Url>() {
            Ok(parsed) => parsed,
            Err(e) => return Err(DbError::Data(format!("Failed to parse URL: {}", e))),
        };

        // Extract the base URL (scheme + host)
        let base_url = match parsed_url.host_str() {
            Some(host) => format!("{}://{}", parsed_url.scheme(), host),
            None => return Err(DbError::Data("URL has no host".to_string())),
        };

        let mut rows = self
            .conn
            .query(
                "SELECT id, url, domain, first_index_date, last_index_date, page_count, status 
             FROM websites 
             WHERE url = ?",
                params![base_url],
            )
            .await
            .map_err(|e| DbError::Query(format!("Failed to get website: {}", e)))?;

        // In libsql 0.6.0, next() is async and returns Result<Option<Row>>
        match rows.next().await {
            Ok(Some(row)) => Ok(Some(self.row_to_website(&row)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(DbError::Data(format!("Failed to get website: {}", e))),
        }
    }

    /// Get all websites
    pub async fn get_all_websites(&self) -> Result<Vec<Website>, DbError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, url, domain, first_index_date, last_index_date, page_count, status 
             FROM websites",
                params![],
            )
            .await
            .map_err(|e| DbError::Query(format!("Failed to get websites: {}", e)))?;

        let mut websites = Vec::new();
        while let Ok(Some(row)) = rows.next().await {
            websites.push(self.row_to_website(&row)?);
        }

        Ok(websites)
    }

    /// List all websites (alias for get_all_websites)
    pub async fn list_websites(&self) -> Result<Vec<Website>, DbError> {
        self.get_all_websites().await
    }

    /// Get websites that need to be crawled
    pub async fn get_websites_to_crawl(&self) -> Result<Vec<Website>, DbError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Get websites that haven't been crawled in the last 7 days
        let mut rows = self
            .conn
            .query(
                "SELECT id, url, domain, first_index_date, last_index_date, page_count, status 
             FROM websites 
             WHERE status = 'active' AND (last_index_date IS NULL OR last_index_date < ?)",
                params![now - 604800], // 7 days in seconds
            )
            .await
            .map_err(|e| DbError::Query(format!("Failed to get websites to crawl: {}", e)))?;

        let mut websites = Vec::new();
        // In libsql 0.6.0, next() is async and returns Result<Option<Row>>
        while let Ok(Some(row)) = rows.next().await {
            websites.push(self.row_to_website(&row)?);
        }

        Ok(websites)
    }

    /// Get a website ID by page URL
    pub async fn get_website_by_page_url(&self, page_url: &str) -> Result<Option<i64>, DbError> {
        // Get the website by URL (this will extract the base URL)
        let website = self.get_website_by_url(page_url).await?;

        // Return the website ID if found
        Ok(website.map(|w| w.id))
    }

    /// Update website last crawled time by URL
    pub async fn update_website_crawl_time_by_url(&self, url: &str) -> Result<(), DbError> {
        // Get the website ID from the URL
        let website_id = match self.get_website_by_page_url(url).await? {
            Some(id) => id,
            None => return Err(DbError::Data(format!("Website not found for URL: {}", url))),
        };

        // Update the website crawl time
        self.update_website_crawl_time(website_id).await
    }

    /// Update website last crawled time
    pub async fn update_website_crawl_time(&self, website_id: i64) -> Result<(), DbError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.conn
            .execute(
                "UPDATE websites SET last_index_date = ? WHERE id = ?",
                params![now, website_id],
            )
            .await
            .map_err(|e| DbError::Query(format!("Failed to update website crawl time: {}", e)))?;

        Ok(())
    }

    /// Update website index with new chunks
    pub async fn update_website_index(
        &self,
        url: &str,
        chunks: Vec<crate::processor::ProcessedChunk>,
    ) -> Result<i64, DbError> {
        // Start a transaction
        let tx = self
            .conn
            .transaction()
            .await
            .map_err(|e| DbError::Transaction(format!("Failed to start transaction: {}", e)))?;

        // Parse the URL to extract the base URL
        let parsed_url = url
            .parse::<url::Url>()
            .map_err(|e| DbError::Data(format!("Failed to parse URL: {}", e)))?;

        // Extract the base URL (scheme + host)
        let base_url = format!(
            "{}://{}",
            parsed_url.scheme(),
            parsed_url
                .host_str()
                .ok_or_else(|| DbError::Data("URL has no host".to_string()))?
        );

        // Get or create the website using the base URL
        let website_id = match self.get_website_by_url(&base_url).await? {
            Some(website) => {
                // Update the website
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;

                tx.execute(
                    "UPDATE websites SET last_index_date = ?, page_count = page_count + 1 WHERE id = ?",
                    params![now, website.id],
                ).await.map_err(|e| DbError::Query(format!("Failed to update website: {}", e)))?;

                website.id
            }
            None => {
                // Create a new website
                let domain = parsed_url
                    .host_str()
                    .ok_or_else(|| DbError::Data("URL has no host".to_string()))?
                    .to_string();

                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;

                let website = Website {
                    id: 0, // Will be set by the database
                    url: base_url,
                    domain,
                    first_index_date: now,
                    last_index_date: now,
                    page_count: 1,
                    status: "active".to_string(),
                };

                // Add the website
                tx.execute(
                    "INSERT INTO websites (url, domain, first_index_date, last_index_date, page_count, status) 
                     VALUES (?, ?, ?, ?, ?, ?)",
                    params![
                        website.url,
                        website.domain,
                        website.first_index_date,
                        website.last_index_date,
                        website.page_count,
                        website.status,
                    ],
                ).await.map_err(|e| DbError::Query(format!("Failed to add website: {}", e)))?;

                // Get the ID of the inserted website
                let mut rows = tx
                    .query("SELECT last_insert_rowid()", params![])
                    .await
                    .map_err(|e| DbError::Query(format!("Failed to get last insert ID: {}", e)))?;

                let row = match rows.next().await {
                    Ok(Some(row)) => row,
                    Ok(None) => {
                        return Err(DbError::Data(
                            "No ID returned from last_insert_rowid()".to_string(),
                        ))
                    }
                    Err(e) => return Err(DbError::Data(format!("Failed to get ID: {}", e))),
                };

                row.get(0)
                    .map_err(|e| DbError::Data(format!("Failed to get ID: {}", e)))?
            }
        };

        // Delete existing chunks for this URL
        tx.execute("DELETE FROM chunks WHERE url = ?", params![url])
            .await
            .map_err(|e| DbError::Query(format!("Failed to delete chunks: {}", e)))?;

        // Add new chunks
        for chunk in chunks {
            let indexed_chunk = IndexedChunk {
                id: 0, // Will be set by the database
                website_id,
                url: url.to_string(),
                text: chunk.text,
                summary: chunk.summary,
                context: chunk.context,
                embedding: chunk.embedding,
                position: chunk.metadata.position as i64,
                heading: chunk.metadata.heading,
            };

            // Insert the chunk with the embedding as a binary blob
            tx.execute(
                "INSERT INTO chunks (website_id, url, text, summary, context, embedding, position, heading) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    indexed_chunk.website_id,
                    indexed_chunk.url,
                    indexed_chunk.text,
                    indexed_chunk.summary,
                    indexed_chunk.context,
                    libsql::Value::Blob(indexed_chunk.embedding.iter().flat_map(|f| f.to_le_bytes()).collect()),
                    indexed_chunk.position,
                    indexed_chunk.heading,
                ],
            ).await.map_err(|e| DbError::Query(format!("Failed to add chunk: {}", e)))?;
        }

        // Commit the transaction
        tx.commit()
            .await
            .map_err(|e| DbError::Transaction(format!("Failed to commit transaction: {}", e)))?;

        Ok(website_id)
    }

    /// Add a chunk to the index
    pub async fn add_chunk(&self, chunk: &IndexedChunk) -> Result<i64, DbError> {
        // Insert the chunk with the embedding as a binary blob
        self.conn.execute(
            "INSERT INTO chunks (website_id, url, text, summary, context, embedding, position, heading) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                chunk.website_id,
                chunk.url.clone(),
                chunk.text.clone(),
                chunk.summary.clone(),
                chunk.context.clone(),
                libsql::Value::Blob(chunk.embedding.iter().flat_map(|f| f.to_le_bytes()).collect()),
                chunk.position,
                chunk.heading.clone(),
            ],
        ).await.map_err(|e| DbError::Query(format!("Failed to add chunk: {}", e)))?;

        // Get the ID of the inserted chunk
        let mut rows = self
            .conn
            .query("SELECT last_insert_rowid()", params![])
            .await
            .map_err(|e| DbError::Query(format!("Failed to get last insert ID: {}", e)))?;

        // In libsql 0.6.0, next() is async and returns Result<Option<Row>>
        let row = match rows.next().await {
            Ok(Some(row)) => row,
            Ok(None) => {
                return Err(DbError::Data(
                    "No ID returned from last_insert_rowid()".to_string(),
                ))
            }
            Err(e) => return Err(DbError::Data(format!("Failed to get ID: {}", e))),
        };

        let id = row
            .get(0)
            .map_err(|e| DbError::Data(format!("Failed to get ID: {}", e)))?;
        Ok(id)
    }

    /// Get chunks by website URL
    pub async fn get_chunks_by_website_url(&self, url: &str) -> Result<Vec<IndexedChunk>, DbError> {
        // Get the website ID from the URL
        let website_id = match self.get_website_by_page_url(url).await? {
            Some(id) => id,
            None => return Err(DbError::Data(format!("Website not found for URL: {}", url))),
        };

        // Get chunks by website ID
        self.get_chunks_by_website(website_id).await
    }

    /// Get chunks by website ID
    pub async fn get_chunks_by_website(
        &self,
        website_id: i64,
    ) -> Result<Vec<IndexedChunk>, DbError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, website_id, url, text, summary, context, embedding, position, heading 
             FROM chunks 
             WHERE website_id = ?",
                params![website_id],
            )
            .await
            .map_err(|e| DbError::Query(format!("Failed to get chunks: {}", e)))?;

        let mut chunks = Vec::new();
        // In libsql 0.6.0, next() is async and returns Result<Option<Row>>
        while let Ok(Some(row)) = rows.next().await {
            chunks.push(self.row_to_chunk(&row)?);
        }

        Ok(chunks)
    }

    /// Delete chunks by website ID
    pub async fn delete_chunks_by_website(&self, website_id: i64) -> Result<usize, DbError> {
        self.conn
            .execute(
                "DELETE FROM chunks WHERE website_id = ?",
                params![website_id],
            )
            .await
            .map_err(|e| DbError::Query(format!("Failed to delete chunks: {}", e)))?;

        // Get the number of affected rows
        let mut count_rows = self
            .conn
            .query("SELECT changes()", params![])
            .await
            .map_err(|e| DbError::Query(format!("Failed to get changes count: {}", e)))?;

        // In libsql 0.6.0, next() is async and returns Result<Option<Row>>
        let row = match count_rows.next().await {
            Ok(Some(row)) => row,
            Ok(None) => return Ok(0),
            Err(e) => return Err(DbError::Data(format!("Failed to get count: {}", e))),
        };

        let count: i64 = row
            .get(0)
            .map_err(|e| DbError::Data(format!("Failed to get count: {}", e)))?;
        Ok(count as usize)
    }

    /// Delete chunks by website URL
    pub async fn delete_chunks_by_website_url(&self, url: &str) -> Result<usize, DbError> {
        // Get the website ID from the URL
        let website_id = match self.get_website_by_page_url(url).await? {
            Some(id) => id,
            None => return Err(DbError::Data(format!("Website not found for URL: {}", url))),
        };

        // Delete chunks by website ID
        self.delete_chunks_by_website(website_id).await
    }

    /// Convert a database row to a Website
    fn row_to_website(&self, row: &Row) -> Result<Website, DbError> {
        Ok(Website {
            id: row
                .get(0)
                .map_err(|e| DbError::Data(format!("Failed to get id: {}", e)))?,
            url: row
                .get(1)
                .map_err(|e| DbError::Data(format!("Failed to get url: {}", e)))?,
            domain: row
                .get(2)
                .map_err(|e| DbError::Data(format!("Failed to get domain: {}", e)))?,
            first_index_date: row
                .get(3)
                .map_err(|e| DbError::Data(format!("Failed to get first_index_date: {}", e)))?,
            last_index_date: row
                .get(4)
                .map_err(|e| DbError::Data(format!("Failed to get last_index_date: {}", e)))?,
            page_count: row
                .get(5)
                .map_err(|e| DbError::Data(format!("Failed to get page_count: {}", e)))?,
            status: row
                .get(6)
                .map_err(|e| DbError::Data(format!("Failed to get status: {}", e)))?,
        })
    }

    /// Convert a database row to an IndexedChunk
    fn row_to_chunk(&self, row: &Row) -> Result<IndexedChunk, DbError> {
        // Get the embedding as a binary blob and convert it to Vec<f32>
        let embedding_blob: Vec<u8> = row
            .get(6)
            .map_err(|e| DbError::Data(format!("Failed to get embedding: {}", e)))?;

        // Convert the blob to Vec<f32>
        let mut embedding = Vec::with_capacity(embedding_blob.len() / 4);
        for chunk in embedding_blob.chunks_exact(4) {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(chunk);
            embedding.push(f32::from_le_bytes(bytes));
        }

        Ok(IndexedChunk {
            id: row
                .get(0)
                .map_err(|e| DbError::Data(format!("Failed to get id: {}", e)))?,
            website_id: row
                .get(1)
                .map_err(|e| DbError::Data(format!("Failed to get website_id: {}", e)))?,
            url: row
                .get(2)
                .map_err(|e| DbError::Data(format!("Failed to get url: {}", e)))?,
            text: row
                .get(3)
                .map_err(|e| DbError::Data(format!("Failed to get text: {}", e)))?,
            summary: row
                .get(4)
                .map_err(|e| DbError::Data(format!("Failed to get summary: {}", e)))?,
            context: row
                .get(5)
                .map_err(|e| DbError::Data(format!("Failed to get context: {}", e)))?,
            embedding,
            position: row
                .get(7)
                .map_err(|e| DbError::Data(format!("Failed to get position: {}", e)))?,
            heading: row
                .get(8)
                .map_err(|e| DbError::Data(format!("Failed to get heading: {}", e)))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::tempdir;

    async fn setup_test_db() -> Result<(Database, tempfile::TempDir), DbError> {
        // Create a temporary directory for the database
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_string_lossy()
            .to_string();

        // Create a new database
        let db = Database::new_from_path(&db_path).await?;

        Ok((db, temp_dir))
    }

    #[tokio::test]
    async fn test_database_initialization() {
        let (db, _temp_dir) = setup_test_db().await.unwrap();

        // Verify that the tables were created
        let mut result = db.execute_query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('websites', 'chunks')",
            params![],
        ).await.unwrap();

        let mut tables = Vec::new();
        while let Ok(Some(row)) = result.next().await {
            let table_name: String = row.get(0).unwrap();
            tables.push(table_name);
        }

        assert_eq!(tables.len(), 2);
        assert!(tables.contains(&"websites".to_string()));
        assert!(tables.contains(&"chunks".to_string()));
    }

    #[tokio::test]
    async fn test_add_and_get_website() {
        let (db, _temp_dir) = setup_test_db().await.unwrap();

        // Create a test website
        let website = Website {
            id: 0, // Will be set by the database
            url: "https://example.com".to_string(),
            domain: "example.com".to_string(),
            first_index_date: 1625097600,
            last_index_date: 1625097600,
            page_count: 10,
            status: "active".to_string(),
        };

        // Add the website
        let id = db.add_website(&website).await.unwrap();
        assert!(id > 0);

        // Get the website by URL
        let retrieved = db
            .get_website_by_url("https://example.com")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.url, "https://example.com");
        assert_eq!(retrieved.domain, "example.com");
        assert_eq!(retrieved.first_index_date, 1625097600);
        assert_eq!(retrieved.last_index_date, 1625097600);
        assert_eq!(retrieved.page_count, 10);
        assert_eq!(retrieved.status, "active");
    }

    #[tokio::test]
    async fn test_get_all_websites() {
        let (db, _temp_dir) = setup_test_db().await.unwrap();

        // Create test websites
        let websites = vec![
            Website {
                id: 0,
                url: "https://example1.com".to_string(),
                domain: "example1.com".to_string(),
                first_index_date: 1625097600,
                last_index_date: 1625097600,
                page_count: 10,
                status: "active".to_string(),
            },
            Website {
                id: 0,
                url: "https://example2.com".to_string(),
                domain: "example2.com".to_string(),
                first_index_date: 1625097700,
                last_index_date: 1625097700,
                page_count: 5,
                status: "active".to_string(),
            },
        ];

        // Add the websites
        for website in &websites {
            db.add_website(website).await.unwrap();
        }

        // Get all websites
        let retrieved = db.get_all_websites().await.unwrap();

        assert_eq!(retrieved.len(), 2);
        assert_eq!(retrieved[0].url, "https://example1.com");
        assert_eq!(retrieved[1].url, "https://example2.com");
    }
}
