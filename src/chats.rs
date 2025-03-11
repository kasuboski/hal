//! Chats service for the HAL crate
//!
//! This module provides functionality for multi-turn conversations with Gemini models.

use crate::error::Result;
use crate::http::HttpClient;
use crate::types::{Content, GenerateContentResponse, GenerationConfig, SafetySetting};
use serde::Serialize;
use tracing::{debug, instrument};

/// Request for creating a chat session
#[derive(Debug, Serialize)]
struct CreateChatRequest {
    /// Model to use for the chat
    model: String,
    
    /// Generation configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
    
    /// Safety settings
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_settings: Option<Vec<SafetySetting>>,
}

/// Request for sending a message in a chat
#[derive(Debug, Serialize)]
struct SendMessageRequest {
    /// The message content
    content: Content,
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
        
        let request = CreateChatRequest {
            model: model.clone(),
            generation_config: config,
            safety_settings,
        };
        
        let path = "chats";
        
        debug!("Creating chat session with model {}", model);
        let response = self.http_client.post::<serde_json::Value, _>(path, &request, self.is_vertex).await?;
        
        // Extract chat ID from response
        let chat_id = response["name"]
            .as_str()
            .ok_or_else(|| crate::error::Error::UnexpectedResponse("Missing chat ID in response".to_string()))?;
        
        Ok(ChatSession {
            chat_id: chat_id.to_string(),
            model,
            http_client: self.http_client.clone(),
            is_vertex: self.is_vertex,
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
}

impl ChatSession {
    /// Send a message in this chat session
    #[instrument(skip(self, message), level = "debug", fields(chat_id = %self.chat_id))]
    pub async fn send_message(
        &self,
        message: impl Into<String> + std::fmt::Debug,
    ) -> Result<GenerateContentResponse> {
        let content = Content::new().with_role("user").with_text(message.into());
        
        let request = SendMessageRequest {
            content,
        };
        
        // The chat_id already contains the 'chats/' prefix, so we don't need to add it again
        let path = format!("{}/messages", self.chat_id);
        
        debug!("Sending message in chat {}", self.chat_id);
        self.http_client.post(&path, &request, self.is_vertex).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // No need to import unused types
    use mockito::Server;
    
    #[tokio::test]
    async fn test_create_chat() {
        let mut server = Server::new_async().await;
        let mock_server = server.mock("POST", "/v1beta/chats")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "name": "chats/test-chat-id"
            }"#)
            .match_query(mockito::Matcher::Any)
            .expect(1)
            .create_async().await;
            
        let mut http_client = HttpClient::with_api_key("test-key".to_string());
        http_client.set_base_url(server.url());
        
        let chats_service = ChatsService::new(http_client, false);
        
        let chat = chats_service.create("gemini-pro").await.unwrap();
        assert_eq!(chat.chat_id, "chats/test-chat-id");
        
        mock_server.assert_async().await;
    }
    
    #[tokio::test]
    async fn test_send_message() {
        let mut server = Server::new_async().await;
        let mock_server = server.mock("POST", "/v1beta/chats/test-chat-id/messages")
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
        };
        
        let response = chat.send_message("Hello").await.unwrap();
        assert_eq!(response.text(), "Response text");
        
        mock_server.assert_async().await;
    }
}