//! Chats service for the HAL crate
//!
//! This module provides functionality for multi-turn conversations with Gemini models.

use crate::error::Result;
use crate::gemini::types::{Content, GenerateContentResponse, GenerationConfig, SafetySetting};
use crate::gemini::models::ModelsService;
use tracing::{debug, instrument};

/// Service for chat sessions
#[derive(Clone)]
pub struct ChatsService {
    /// Models service for making API requests
    models: ModelsService,
}

impl ChatsService {
    /// Create a new chats service
    pub(crate) fn new(models: ModelsService) -> Self {
        Self {
            models,
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
            models: self.models.clone(),
            generation_config: config,
            safety_settings,
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
    
    /// Models service for making API requests
    models: ModelsService,
    
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
        
        debug!("Sending message using model {} in chat {}", self.model, self.chat_id);
        self.models.generate_content_with_config(
            &self.model,
            contents,
            self.generation_config.clone(),
            self.safety_settings.clone()
        ).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use crate::gemini::http::HttpClient;
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
        
        let models = ModelsService::new(http_client, false);
        
        let chat = ChatSession {
            chat_id: "chats/test-chat-id".to_string(),
            model: "gemini-pro".to_string(),
            models,
            generation_config: None,
            safety_settings: None,
        };
        
        let response = chat.send_message("Hello", None).await.unwrap();
        assert_eq!(response.text(), "Response text");
        
        mock_server.assert_async().await;
    }
}