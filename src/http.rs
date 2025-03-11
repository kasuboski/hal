//! HTTP client implementation for the HAL crate
//!
//! This module provides the HTTP client for making requests to the Gemini API.

use crate::error::{Error, Result};
use crate::types::HttpOptions;
use reqwest::{Client as ReqwestClient, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;
use tracing::{debug, error, instrument};
use url::Url;

/// Default timeout for HTTP requests in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// HTTP client for making requests to the Gemini API
#[derive(Clone)]
pub struct HttpClient {
    /// The underlying reqwest client
    client: ReqwestClient,
    
    /// Base URL for API requests
    base_url: String,
    
    /// API key for authentication (Gemini Developer API)
    api_key: Option<String>,
    
    /// Project ID for Vertex AI
    project_id: Option<String>,
    
    /// Location for Vertex AI
    location: Option<String>,
    
    /// API version
    api_version: String,
}

#[cfg(test)]
impl HttpClient {
    /// Set the base URL (for testing only)
    pub fn set_base_url(&mut self, url: String) {
        self.base_url = url;
    }
}

impl HttpClient {
    /// Create a new HTTP client with an API key for the Gemini Developer API
    pub fn with_api_key(api_key: String) -> Self {
        Self::with_api_key_and_options(api_key, HttpOptions::default())
    }
    
    /// Create a new HTTP client with an API key and custom options
    pub fn with_api_key_and_options(api_key: String, options: HttpOptions) -> Self {
        let client = ReqwestClient::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");
            
        Self {
            client,
            base_url: "https://generativelanguage.googleapis.com".to_string(),
            api_key: Some(api_key),
            project_id: None,
            location: None,
            api_version: options.api_version,
        }
    }
    
    /// Create a new HTTP client for Vertex AI
    pub fn with_vertex_ai(project_id: String, location: String) -> Self {
        Self::with_vertex_ai_and_options(project_id, location, HttpOptions::default())
    }
    
    /// Create a new HTTP client for Vertex AI with custom options
    pub fn with_vertex_ai_and_options(project_id: String, location: String, options: HttpOptions) -> Self {
        let client = ReqwestClient::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");
            
        Self {
            client,
            base_url: format!("https://{}-aiplatform.googleapis.com", location),
            api_key: None,
            project_id: Some(project_id),
            location: Some(location),
            api_version: options.api_version,
        }
    }
    
    /// Build a URL for the Gemini Developer API
    fn build_genai_url(&self, path: &str) -> Result<Url> {
        let url = format!("{}/{}/{}", self.base_url, self.api_version, path);
        Url::parse(&url).map_err(|e| Error::Other(format!("Invalid URL: {}", e)))
    }
    
    /// Build a URL for Vertex AI
    fn build_vertex_url(&self, path: &str) -> Result<Url> {
        if let (Some(project_id), Some(location)) = (&self.project_id, &self.location) {
            let url = format!("{}/{}/projects/{}/locations/{}/{}", 
                self.base_url, 
                self.api_version, 
                project_id, 
                location, 
                path
            );
            Url::parse(&url).map_err(|e| Error::Other(format!("Invalid URL: {}", e)))
        } else {
            Err(Error::Auth("Missing project_id or location for Vertex AI".to_string()))
        }
    }
    
    /// Prepare a GET request
    #[instrument(skip(self), level = "debug")]
    pub async fn get<T: DeserializeOwned>(&self, path: &str, is_vertex: bool) -> Result<T> {
        let url = if is_vertex {
            self.build_vertex_url(path)?
        } else {
            self.build_genai_url(path)?
        };
        
        let mut request = self.client.get(url);
        
        if let Some(api_key) = &self.api_key {
            request = request.query(&[("key", api_key)]);
        }
        
        debug!("Sending GET request to {}", path);
        self.execute_request(request).await
    }
    
    /// Prepare a POST request with a JSON body
    #[instrument(skip(self, body), level = "debug")]
    pub async fn post<T: DeserializeOwned, B: Serialize + std::fmt::Debug>(
        &self, 
        path: &str, 
        body: &B, 
        is_vertex: bool
    ) -> Result<T> {
        let url = if is_vertex {
            self.build_vertex_url(path)?
        } else {
            self.build_genai_url(path)?
        };
        
        let mut request = self.client.post(url).json(body);
        
        if let Some(api_key) = &self.api_key {
            request = request.query(&[("key", api_key)]);
        }
        
        debug!("Sending POST request to {}", path);
        self.execute_request(request).await
    }
    
    /// Prepare a DELETE request
    #[instrument(skip(self), level = "debug")]
    pub async fn delete<T: DeserializeOwned>(&self, path: &str, is_vertex: bool) -> Result<T> {
        let url = if is_vertex {
            self.build_vertex_url(path)?
        } else {
            self.build_genai_url(path)?
        };
        
        let mut request = self.client.delete(url);
        
        if let Some(api_key) = &self.api_key {
            request = request.query(&[("key", api_key)]);
        }
        
        debug!("Sending DELETE request to {}", path);
        self.execute_request(request).await
    }
    
    /// Execute an HTTP request and handle the response
    async fn execute_request<T: DeserializeOwned>(&self, request: RequestBuilder) -> Result<T> {
        let response = request.send().await.map_err(Error::Http)?;
        
        let status = response.status();
        let response_text = response.text().await.map_err(Error::Http)?;
        
        if status.is_success() {
            serde_json::from_str(&response_text).map_err(|e| {
                error!("Failed to parse response: {}", e);
                Error::UnexpectedResponse(format!("Failed to parse response: {}", e))
            })
        } else {
            error!("API error: {} - {}", status, response_text);
            
            if status == StatusCode::TOO_MANY_REQUESTS {
                Err(Error::RateLimit { retry_after_secs: 60 })
            } else if status == StatusCode::UNAUTHORIZED {
                Err(Error::Auth("Invalid API key or credentials".to_string()))
            } else if status == StatusCode::NOT_IMPLEMENTED {
                Err(Error::Unsupported(format!("Operation not supported: {}", response_text)))
            } else {
                Err(Error::Api {
                    status_code: status.as_u16(),
                    message: response_text,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use serde::Deserialize;
    
    #[derive(Debug, Deserialize)]
    struct TestResponse {
        message: String,
    }
    
    #[tokio::test]
    async fn test_get_request_success() {
        let mut server = Server::new_async().await;
        let mock_server = server.mock("GET", "/v1beta/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{\"message\": \"success\"}")
            .match_query(mockito::Matcher::Any)
            .expect(1)
            .create_async().await;
            
        let mut client = HttpClient::with_api_key("test-key".to_string());
        client.set_base_url(server.url());
        
        let response: TestResponse = client.get("test", false).await.unwrap();
        assert_eq!(response.message, "success");
        
        mock_server.assert_async().await;
    }
    
    #[tokio::test]
    async fn test_post_request_success() {
        let mut server = Server::new_async().await;
        let mock_server = server.mock("POST", "/v1beta/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{\"message\": \"success\"}")
            .match_query(mockito::Matcher::Any)
            .expect(1)
            .create_async().await;
            
        let mut client = HttpClient::with_api_key("test-key".to_string());
        client.set_base_url(server.url());
        
        let body = serde_json::json!({"test": "data"});
        let response: TestResponse = client.post("test", &body, false).await.unwrap();
        assert_eq!(response.message, "success");
        
        mock_server.assert_async().await;
    }
    
    #[tokio::test]
    async fn test_error_handling() {
        let mut server = Server::new_async().await;
        let mock_server = server.mock("GET", "/v1beta/test")
            .with_status(501)
            .with_body("Not Implemented")
            .match_query(mockito::Matcher::Any)
            .create_async().await;
            
        let mut client = HttpClient::with_api_key("test-key".to_string());
        client.set_base_url(server.url());
        
        let result: Result<TestResponse> = client.get("test", false).await;
        assert!(matches!(result, Err(Error::Unsupported(_))));
        
        mock_server.assert_async().await;
    }
}