//! # Mock Completion Model for Testing
//!
//! Provides a `MockCompletionModel` that implements the `CompletionModel` trait
//! for use in tests. It allows setting a predefined response or error to simulate
//! different model behaviors without making actual API calls.

use rig::{
    completion::{
        AssistantContent, CompletionError, CompletionModel, CompletionRequest, CompletionResponse,
    },
    one_or_many::OneOrMany,
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// A mock completion model for testing purposes.
/// It returns a predefined response or error when `completion` is called.
#[derive(Debug, Clone)]
pub struct MockCompletionModel {
    /// The predefined response to return. Arc<Mutex<>> allows modification after creation if needed.
    response: Arc<Mutex<Option<OneOrMany<AssistantContent>>>>,
}

impl MockCompletionModel {
    /// Creates a new mock model that will return a default empty success response.
    pub fn new() -> Self {
        Self {
            response: Arc::new(Mutex::new(None)),
        }
    }

    /// Sets the response that the mock model should return.
    pub async fn set_response(&self, response: OneOrMany<AssistantContent>) {
        let mut guard = self.response.lock().await;
        *guard = Some(response);
    }

    /// Helper to create a simple text response.
    pub async fn set_text_response(&self, text: &str) {
        let response = OneOrMany::one(AssistantContent::text(text));
        self.set_response(response).await;
    }
}

impl Default for MockCompletionModel {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionModel for MockCompletionModel {
    type Response = String;

    async fn completion(
        &self,
        _completion_request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        let response = {
            let guard = self.response.lock().await;
            guard.clone()
        };
        match response {
            Some(result) => Ok(CompletionResponse {
                choice: result,
                raw_response: "".to_string(),
            }),
            None => Ok(CompletionResponse {
                choice: OneOrMany::one(AssistantContent::text("")),
                raw_response: "".to_string(),
            }),
        }
    }
}
