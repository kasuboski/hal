//! Models service for the HAL crate
//!
//! This module provides functionality for interacting with Gemini models,
//! including content generation, embedding, and token counting.

use crate::error::Result;
use crate::gemini::types::{Content, CountTokensResponse, EmbedContentResponse, GenerateContentResponse, GenerationConfig, SafetySetting};
use crate::gemini::http::HttpClient;
use async_trait::async_trait;
use serde::Serialize;
use tracing::{debug, instrument};

/// Request for generating content
#[derive(Debug, Serialize)]
struct GenerateContentRequest {
    /// The contents to generate from
    contents: Vec<Content>,
    
    /// Generation configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
    
    /// Safety settings
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_settings: Option<Vec<SafetySetting>>,

    /// The system prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<Content>,
}

/// Request for counting tokens
#[derive(Debug, Serialize)]
struct CountTokensRequest {
    /// The contents to count tokens for
    contents: Vec<Content>,
}

/// Request for embedding content
#[derive(Debug, Serialize)]
struct EmbedContentRequest {
    /// The content to embed
    content: Content,
}

/// Service for interacting with Gemini models
#[derive(Clone)]
pub struct ModelsService {
    /// HTTP client for making API requests
    http_client: HttpClient,
    
    /// Whether this service is using Vertex AI
    is_vertex: bool,
}

impl ModelsService {
    /// Create a new models service
    pub(crate) fn new(http_client: HttpClient, is_vertex: bool) -> Self {
        Self {
            http_client,
            is_vertex,
        }
    }
    
    /// Generate content from a model
    #[instrument(skip(self, contents), level = "debug")]
    pub async fn generate_content(
        &self,
        model: impl Into<String> + std::fmt::Debug,
        system_instruction: Option<Content>,
        contents: Vec<Content>,
    ) -> Result<GenerateContentResponse> {
        self.generate_content_with_config(model, system_instruction, contents, None, None).await
    }
    
    /// Generate content with configuration
    #[instrument(skip(self, contents, config, safety_settings), level = "debug")]
    pub async fn generate_content_with_config(
        &self,
        model: impl Into<String> + std::fmt::Debug,
        system_instruction: Option<Content>,
        contents: Vec<Content>,
        config: Option<GenerationConfig>,
        safety_settings: Option<Vec<SafetySetting>>,
    ) -> Result<GenerateContentResponse> {
        let model = model.into();
        
        let request = GenerateContentRequest {
            contents,
            generation_config: config,
            safety_settings,
            system_instruction,
        };
        
        let path = format!("models/{}:generateContent", model);
        
        debug!("Generating content from model {}", model);
        self.http_client.post(&path, &request, self.is_vertex).await
    }
    
    /// Count tokens in content
    #[instrument(skip(self, contents), level = "debug")]
    pub async fn count_tokens(
        &self,
        model: impl Into<String> + std::fmt::Debug,
        contents: Vec<Content>,
    ) -> Result<CountTokensResponse> {
        let model = model.into();
        
        let request = CountTokensRequest {
            contents,
        };
        
        let path = format!("models/{}:countTokens", model);
        
        debug!("Counting tokens for model {}", model);
        self.http_client.post(&path, &request, self.is_vertex).await
    }
    
    /// Generate embeddings from content
    #[instrument(skip(self, contents), level = "debug")]
    pub async fn embed_content(
        &self,
        model: impl Into<String> + std::fmt::Debug,
        contents: impl Into<Content>,
    ) -> Result<EmbedContentResponse> {
        let model = model.into();
        let content = contents.into();
        
        let request = EmbedContentRequest {
            content,
        };
        
        let path = format!("models/{}/embedContent", model);
        
        debug!("Generating embeddings from model {}", model);
        self.http_client.post(&path, &request, self.is_vertex).await
    }
    
    /// Compute tokens (Vertex AI only)
    #[instrument(skip(self, contents), level = "debug")]
    pub async fn compute_tokens(
        &self,
        model: impl Into<String> + std::fmt::Debug,
        contents: Vec<Content>,
    ) -> Result<CountTokensResponse> {
        if !self.is_vertex {
            return Err(crate::error::Error::Unsupported(
                "compute_tokens is only available in Vertex AI".to_string(),
            ));
        }
        
        let model = model.into();
        
        let request = CountTokensRequest {
            contents,
        };
        
        let path = format!("models/{}/computeTokens", model);
        
        debug!("Computing tokens for model {}", model);
        self.http_client.post(&path, &request, true).await
    }
}

/// Trait for streaming content generation
#[async_trait]
pub trait StreamingModelsService {
    /// Stream generated content
    async fn generate_content_stream(
        &self,
        model: impl Into<String> + Send + std::fmt::Debug,
        contents: impl Into<Content> + Send,
    ) -> Result<impl futures::Stream<Item = Result<GenerateContentResponse>>>;
    
    /// Stream generated content with configuration
    async fn generate_content_stream_with_config(
        &self,
        model: impl Into<String> + Send + std::fmt::Debug,
        contents: impl Into<Content> + Send,
        config: Option<GenerationConfig>,
        safety_settings: Option<Vec<SafetySetting>>,
    ) -> Result<impl futures::Stream<Item = Result<GenerateContentResponse>>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    
    #[tokio::test]
    async fn test_generate_content() {
        let mut server = Server::new_async().await;
        let mock_server = server.mock("POST", "/v1beta/models/gemini-pro:generateContent")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "candidates": [{
                    "content": {
                        "parts": [{
                            "text": "Generated text"
                        }]
                    }
                }]
            }"#)
            .create_async().await;
            
        let mut http_client = HttpClient::with_api_key("test-key".to_string());
        http_client.set_base_url(server.url());
        
        let models_service = ModelsService::new(http_client, false);

        let system = Content::new().with_text("You are a helpful assistant.");
        let content = Content::new().with_text("Hello, world!");
        let response = models_service.generate_content("gemini-pro", Some(system), vec![content]).await.unwrap();
        
        assert_eq!(response.text(), "Generated text");
        mock_server.assert_async().await;
    }
    
    #[tokio::test]
    async fn test_count_tokens() {
        let mut server = Server::new_async().await;
        let mock_server = server.mock("POST", "/v1beta/models/gemini-pro:countTokens")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "total_tokens": 5
            }"#)
            .create_async().await;
            
        let mut http_client = HttpClient::with_api_key("test-key".to_string());
        http_client.set_base_url(server.url());
        
        let models_service = ModelsService::new(http_client, false);
        
        let content = Content::new().with_text("Hello, world!");
        let response = models_service.count_tokens("gemini-pro", vec![content]).await.unwrap();
        
        assert_eq!(response.total_tokens, 5);
        mock_server.assert_async().await;
    }
}