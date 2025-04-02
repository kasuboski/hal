//! # Rate-Limited Completion Model Module
//!
//! This module provides a wrapper around any completion model that adds rate limiting
//! capabilities to prevent API quota exhaustion and manage costs.
//!
//! ## Features
//!
//! - Transparent rate limiting for any model implementing the `CompletionModel` trait
//! - Configurable rate limits with governor crate integration
//! - Instrumentation with tracing spans for monitoring and debugging
//! - Direct integration with the agent framework for conversation management
//!
//! ## Usage
//!
//! The rate-limited model can be configured with different quota settings for
//! different usage tiers (standard vs free). When the rate limit is reached,
//! the model will transparently wait until requests are allowed again.

use std::sync::Arc;

use governor::DefaultDirectRateLimiter;
use rig::{
    agent::AgentBuilder,
    completion::{self, CompletionError, CompletionModel, CompletionRequest, CompletionResponse},
};
use tracing::{Instrument, debug_span, info_span};

use super::RateLimitResponse;

#[derive(Clone)]
pub struct RateLimitedCompletionModel<M: CompletionModel> {
    model: M,
    limiter: Arc<DefaultDirectRateLimiter>,
}

impl<M> RateLimitedCompletionModel<M>
where
    M: CompletionModel,
{
    pub fn new(model: M, limiter: DefaultDirectRateLimiter) -> Self {
        Self {
            model,
            limiter: Arc::new(limiter),
        }
    }

    pub fn agent(self) -> AgentBuilder<Self> {
        AgentBuilder::new(self)
    }
}

impl<M: CompletionModel> CompletionModel for RateLimitedCompletionModel<M> {
    type Response = RateLimitResponse<M::Response>;

    async fn completion(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<completion::CompletionResponse<Self::Response>, CompletionError> {
        self.limiter
            .until_ready()
            .instrument(debug_span!("limiter"))
            .await;
        let request = completion_request_debug(&completion_request);
        let response = self
            .model
            .completion(completion_request)
            .instrument(info_span!(
                "completion",
                prompt = request.0,
                history = request.1
            ))
            .await;
        response.map(|response| {
            let rate_limit = RateLimitResponse {
                response: response.raw_response,
            };
            let choice = response.choice;
            CompletionResponse {
                choice,
                raw_response: rate_limit,
            }
        })
    }
}

/// Return the prompt and history for debugging purposes
fn completion_request_debug(request: &CompletionRequest) -> (String, String) {
    let prompt = request.prompt.clone();
    let history = request.chat_history.clone();
    (format!("{prompt:?}"), format!("{history:?}"))
}
