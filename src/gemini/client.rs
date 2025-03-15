//! Client implementation for the HAL crate
//!
//! This module provides the main client interface for interacting with the Gemini API.

use crate::gemini::batches::BatchesService;
use crate::gemini::caches::CachesService;
use crate::gemini::chats::ChatsService;
// Error and Result are used in the panic message
use crate::gemini::files::FilesService;
use crate::gemini::http::HttpClient;
use crate::gemini::models::ModelsService;
use crate::gemini::types::HttpOptions;

/// Client for the Gemini API
///
/// This is the main entry point for interacting with the Gemini API.
/// It provides access to the various services for models, chats, files, etc.
#[derive(Clone)]
pub struct Client {
    http_client: HttpClient,
    vertexai: bool,
}

impl Client {
    pub fn default_from_client(client: &Client) -> Self {
        let http_client = client.http_client.reset_default_options();
        Self {
            http_client,
            vertexai: client.vertexai,
        }
    }

    /// Create a new client with an API key for the Gemini Developer API
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        let http_client = HttpClient::with_api_key(api_key.into());
        Self {
            http_client,
            vertexai: false,
        }
    }

    /// Create a new client for Vertex AI
    ///
    /// This requires GCP authentication to be set up in the environment.
    pub fn with_vertex_ai(project_id: impl Into<String>, location: impl Into<String>) -> Self {
        let http_client = HttpClient::with_vertex_ai(project_id.into(), location.into());
        Self {
            http_client,
            vertexai: true,
        }
    }

    /// Create a new client with custom HTTP options
    pub fn with_options(
        api_key: Option<String>,
        project_id: Option<String>,
        location: Option<String>,
        options: HttpOptions,
    ) -> Self {
        // Check if project_id is Some before using it in pattern matching to avoid moving it
        let is_vertex = project_id.is_some();

        let http_client = if let Some(api_key) = api_key {
            HttpClient::with_api_key_and_options(api_key, options)
        } else if let (Some(project_id), Some(location)) = (project_id, location) {
            HttpClient::with_vertex_ai_and_options(project_id, location, options)
        } else {
            panic!("Either API key or project_id and location must be provided");
        };

        Self {
            http_client,
            vertexai: is_vertex,
        }
    }

    /// Access the models service
    pub fn models(&self) -> ModelsService {
        ModelsService::new(self.http_client.clone(), self.vertexai)
    }

    /// Access the chats service
    pub fn chats(&self) -> ChatsService {
        ChatsService::new(self.models())
    }

    /// Access the files service
    pub fn files(&self) -> FilesService {
        if !self.vertexai {
            FilesService::new(self.http_client.clone())
        } else {
            panic!("Files service is only available for Gemini Developer API");
        }
    }

    /// Access the caches service
    pub fn caches(&self) -> CachesService {
        CachesService::new(self.http_client.clone(), self.vertexai)
    }

    /// Access the batches service
    pub fn batches(&self) -> BatchesService {
        if self.vertexai {
            BatchesService::new(self.http_client.clone())
        } else {
            panic!("Batches service is only available for Vertex AI");
        }
    }

    /// Check if this client is using Vertex AI
    pub fn is_vertex_ai(&self) -> bool {
        self.vertexai
    }

    /// Create a new client with an API key for the Gemini Developer API with default rate limits
    ///
    /// This is a convenience method that creates a client with client-side rate limiting enabled
    /// to stay within Gemini's default limit of 30 requests per minute per model.
    ///
    /// # Examples
    ///
    /// ```
    /// use hal::gemini::Client;
    ///
    /// let client = Client::with_api_key_rate_limited("your-api-key");
    /// ```
    pub fn with_api_key_rate_limited(api_key: impl Into<String>) -> Self {
        let http_client = HttpClient::with_gemini_rate_limits(api_key.into());
        Self {
            http_client,
            vertexai: false,
        }
    }

    /// Create a new client for Vertex AI with default rate limits
    ///
    /// This is a convenience method that creates a client with client-side rate limiting enabled
    /// to stay within Gemini's default limit of 30 requests per minute per model.
    ///
    /// # Examples
    ///
    /// ```
    /// use hal::gemini::Client;
    ///
    /// let client = Client::with_vertex_ai_rate_limited("your-project-id", "us-central1");
    /// ```
    pub fn with_vertex_ai_rate_limited(
        project_id: impl Into<String>,
        location: impl Into<String>,
    ) -> Self {
        let http_client =
            HttpClient::with_vertex_ai_rate_limits(project_id.into(), location.into());
        Self {
            http_client,
            vertexai: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation_with_api_key() {
        let client = Client::with_api_key("test-api-key");
        assert!(!client.is_vertex_ai());
    }

    #[test]
    fn test_client_creation_with_vertex_ai() {
        let client = Client::with_vertex_ai("test-project", "us-central1");
        assert!(client.is_vertex_ai());
    }
}
