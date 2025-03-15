//! Files service for the HAL crate
//!
//! This module provides functionality for managing files with the Gemini API.
//! Files can be uploaded, retrieved, and deleted.

use crate::error::Result;
use crate::gemini::http::HttpClient;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

/// Request for uploading a file
#[derive(Debug, Serialize)]
struct UploadFileRequest {
    /// The file data in base64 encoding
    data: String,

    /// MIME type of the file
    #[serde(skip_serializing_if = "Option::is_none")]
    mime_type: Option<String>,
}

/// Response from uploading a file
#[derive(Debug, Deserialize)]
pub struct FileResponse {
    /// The file name/ID
    pub name: String,

    /// The file URI
    pub uri: String,

    /// MIME type of the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Size of the file in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
}

/// Service for managing files
#[derive(Clone)]
pub struct FilesService {
    /// HTTP client for making API requests
    http_client: HttpClient,
}

impl FilesService {
    /// Create a new files service
    pub(crate) fn new(http_client: HttpClient) -> Self {
        Self { http_client }
    }

    /// Upload a file
    ///
    /// # Arguments
    ///
    /// * `file_data` - The file data as bytes
    /// * `mime_type` - Optional MIME type of the file
    ///
    /// # Returns
    ///
    /// A `FileResponse` containing the file information
    #[instrument(skip(self, file_data), level = "debug")]
    pub async fn upload(
        &self,
        file_data: impl AsRef<[u8]>,
        mime_type: Option<impl Into<String> + std::fmt::Debug>,
    ) -> Result<FileResponse> {
        let base64_data = base64::engine::general_purpose::STANDARD.encode(file_data.as_ref());

        let request = UploadFileRequest {
            data: base64_data,
            mime_type: mime_type.map(|m| m.into()),
        };

        debug!("Uploading file");
        self.http_client.post("files:upload", &request, false).await
    }

    /// Get file information
    ///
    /// # Arguments
    ///
    /// * `name` - The file name/ID
    ///
    /// # Returns
    ///
    /// A `FileResponse` containing the file information
    #[instrument(skip(self), level = "debug")]
    pub async fn get(&self, name: impl Into<String> + std::fmt::Debug) -> Result<FileResponse> {
        let name = name.into();
        debug!("Getting file information for {}", name);
        self.http_client
            .get(&format!("files/{}", name), false)
            .await
    }

    /// Delete a file
    ///
    /// # Arguments
    ///
    /// * `name` - The file name/ID
    ///
    /// # Returns
    ///
    /// An empty response on success
    #[instrument(skip(self), level = "debug")]
    pub async fn delete(&self, name: impl Into<String> + std::fmt::Debug) -> Result<()> {
        let name = name.into();
        debug!("Deleting file {}", name);
        self.http_client
            .delete::<serde_json::Value>(&format!("files/{}", name), false)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{gemini::http::HttpClient, prelude::HttpOptions};
    use mockito::Server;

    #[tokio::test]
    async fn test_upload_file() {
        let mut server = Server::new_async().await;
        let m = server.mock("POST", "/v1beta/files:upload")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"name":"files/123","uri":"https://example.com/files/123","mime_type":"application/pdf","size_bytes":12345}"#)
            .match_query(mockito::Matcher::Any)
            .expect(1)
            .create_async().await;

        let client = HttpClient::with_api_key_and_options(
            "test-key".to_string(),
            HttpOptions {
                api_version: "v1beta".to_string(),
                headers: std::collections::HashMap::new(),
                retry_on_rate_limit: false,
                max_retries: 3,
                default_retry_after_secs: 60,
                enable_client_side_rate_limiting: false,
                requests_per_minute: 30,
                wait_when_rate_limited: true,
            },
        );

        // Override base URL for testing
        let mut client = client;
        client.set_base_url(server.url());

        let service = FilesService::new(client);
        let response = service
            .upload(b"test file content", Some("application/pdf"))
            .await
            .unwrap();

        assert_eq!(response.name, "files/123");
        assert_eq!(response.uri, "https://example.com/files/123");
        assert_eq!(response.mime_type, Some("application/pdf".to_string()));
        assert_eq!(response.size_bytes, Some(12345));

        m.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_file() {
        let mut server = Server::new_async().await;
        let m = server.mock("GET", "/v1beta/files/files/123")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"name":"files/123","uri":"https://example.com/files/123","mime_type":"application/pdf","size_bytes":12345}"#)
            .match_query(mockito::Matcher::Any)
            .expect(1)
            .create_async().await;

        let client = HttpClient::with_api_key_and_options(
            "test-key".to_string(),
            HttpOptions {
                api_version: "v1beta".to_string(),
                headers: std::collections::HashMap::new(),
                retry_on_rate_limit: false,
                max_retries: 3,
                default_retry_after_secs: 60,
                enable_client_side_rate_limiting: false,
                requests_per_minute: 30,
                wait_when_rate_limited: true,
            },
        );

        // Override base URL for testing
        let mut client = client;
        client.set_base_url(server.url());

        let service = FilesService::new(client);
        let response = service.get("files/123").await.unwrap();

        assert_eq!(response.name, "files/123");
        assert_eq!(response.uri, "https://example.com/files/123");
        assert_eq!(response.mime_type, Some("application/pdf".to_string()));
        assert_eq!(response.size_bytes, Some(12345));

        m.assert_async().await;
    }

    #[tokio::test]
    async fn test_delete_file() {
        let mut server = Server::new_async().await;
        let m = server
            .mock("DELETE", "/v1beta/files/files/123")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .match_query(mockito::Matcher::Any)
            .expect(1)
            .create_async()
            .await;

        let client = HttpClient::with_api_key_and_options(
            "test-key".to_string(),
            HttpOptions {
                api_version: "v1beta".to_string(),
                headers: std::collections::HashMap::new(),
                retry_on_rate_limit: false,
                max_retries: 3,
                default_retry_after_secs: 60,
                enable_client_side_rate_limiting: false,
                requests_per_minute: 30,
                wait_when_rate_limited: true,
            },
        );

        // Override base URL for testing
        let mut client = client;
        client.set_base_url(server.url());

        let service = FilesService::new(client);
        service.delete("files/123").await.unwrap();

        m.assert_async().await;
    }
}
