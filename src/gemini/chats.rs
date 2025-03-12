//! Chats service for the HAL crate
//!
//! This module provides functionality for multi-turn conversations with Gemini models.

use crate::error::Result;
use crate::gemini::http::HttpClient;
use crate::gemini::types::{Content, GenerateContentResponse, GenerationConfig, SafetySetting};
use serde::Serialize;
use tracing::{debug, instrument};

/// Request for sending a message in a chat
#[derive(Debug, Serialize)]
struct SendMessageRequest {
    /// The message content
    contents: Vec<Content>,
    
    /// Generation configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
    
    /// Safety settings
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_settings: Option<Vec<SafetySetting>>,
}

/// Service for chat sessions
#[derive(Clone)]
pub struct ChatsService {
    /// HTTP client for making API requests
    http_client: HttpClient,
    
    /// Whether this service is using Vertex AI
    is_vertex: bool,
}

impl ChatsService {
    /// Create a new chats service
    pub(crate) fn new(http_client: HttpClient, is_vertex: bool) -> Self {
        Self {
            http_client,
            is_vertex,
        }
    }
    
    /// Create a new chat session
    #[instrument(skip(self), level = "debug")]
    pub async fn create(
        &self,
        model: impl Into<String> + std::fmt::Debug,
    ) -> Result<ChatSession> {
        self.create_with_config(model, None, None).await
    }
    
    /// Create a new chat session with configuration
    #[instrument(skip(self, config, safety_settings), level = "debug")]
    pub async fn create_with_config(
        &self,
        model: impl Into<String> + std::fmt::Debug,
        config: Option<GenerationConfig>,
        safety_settings: Option<Vec<SafetySetting>>,
    ) -> Result<ChatSession> {
        let model = model.into();
        
        debug!("Creating chat session with model {}", model);
        
        // Generate a unique chat ID instead of making an HTTP call
        let chat_id = format!("chats/{}", uuid::Uuid::new_v4());
        
        Ok(ChatSession {
            chat_id,
            model,
            http_client: self.http_client.clone(),
            is_vertex: self.is_vertex,
            generation_config: config,
            safety_settings: safety_settings,
        })
    }
}

/// A chat session for multi-turn conversations
#[derive(Clone)]
pub struct ChatSession {
    /// The chat session ID
    chat_id: String,
    
    /// The model used for this chat
    model: String,
    
    /// HTTP client for making API requests
    http_client: HttpClient,
    
    /// Whether this session is using Vertex AI
    is_vertex: bool,
    
    /// Generation configuration
    generation_config: Option<GenerationConfig>,
    
    /// Safety settings
    safety_settings: Option<Vec<SafetySetting>>,
}

impl ChatSession {
    /// Send a message in this chat session
    #[instrument(skip(self, message), level = "debug", fields(chat_id = %self.chat_id))]
    pub async fn send_message(
        &self,
        message: impl Into<String> + std::fmt::Debug,
        history: Option<Vec<Content>>,
    ) -> Result<GenerateContentResponse> {
        let content = Content::new().with_role("user").with_text(message.into());
        
        let mut contents = history.unwrap_or_default();
        contents.push(content);
        
        let request = SendMessageRequest {
            contents,
            generation_config: self.generation_config.clone(),
            safety_settings: self.safety_settings.clone(),
        };
        
        // Use the model name directly in the path for the API request
        let path = format!("models/{}:generateContent", self.model);
        
        debug!("Sending message using model {} in chat {}", self.model, self.chat_id);
        self.http_client.post(&path, &request, self.is_vertex).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // No need to import unused types
    use mockito::Server;
    
    #[tokio::test]
    async fn test_send_message() {
        let mut server = Server::new_async().await;
        let mock_server = server.mock("POST", "/v1beta/models/gemini-pro:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "candidates": [{
                    "content": {
                        "parts": [{
                            "text": "Response text"
                        }]
                    }
                }]
            }"#)
            .match_query(mockito::Matcher::Any)
            .expect(1)
            .create_async().await;
            
        let mut http_client = HttpClient::with_api_key("test-key".to_string());
        http_client.set_base_url(server.url());
        
        let chat = ChatSession {
            chat_id: "chats/test-chat-id".to_string(),
            model: "gemini-pro".to_string(),
            http_client,
            is_vertex: false,
            generation_config: None,
            safety_settings: None,
        };
        
        let response = chat.send_message("Hello", None).await.unwrap();
        assert_eq!(response.text(), "Response text");
        
        mock_server.assert_async().await;
    }
}