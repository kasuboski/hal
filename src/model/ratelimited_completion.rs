use std::sync::Arc;

use governor::DefaultDirectRateLimiter;
use rig::{
    agent::AgentBuilder,
    completion::{self, CompletionError, CompletionModel, CompletionRequest, CompletionResponse},
};

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
        self.limiter.until_ready().await;
        let response = self.model.completion(completion_request).await;
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
