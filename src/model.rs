//! # LLM Client Module
//!
//! This module provides a unified client interface for working with LLM services,
//! with built-in rate limiting to prevent API quota exhaustion.
//!
//! ## Key Components
//!
//! - `Client`: A unified client that wraps both completion and embedding models
//! - `RateLimitedCompletionModel`: A wrapper that adds rate limiting to any completion model
//! - `RateLimitedEmbeddingModel`: A wrapper that adds rate limiting to any embedding model
//! - `EmbeddingConversion`: Utilities for converting between embedding formats
//!
//! ## Features
//!
//! - Configurable rate limiting with different quotas (standard and free tiers)
//! - Environment variable configuration for API keys
//! - Instrumentation with tracing spans for monitoring
//! - Type-safe model integration with the `rig` framework
//! - Conversion utilities for embedding vectors

use std::num::NonZeroU32;

use governor::{Quota, RateLimiter};
use ratelimited_completion::RateLimitedCompletionModel;
use ratelimited_embedding::RateLimitedEmbeddingModel;
use rig::{completion::CompletionModel, embeddings::EmbeddingModel, providers::gemini};

pub mod embedding;
pub mod ratelimited_completion;
pub mod ratelimited_embedding;

pub use embedding::EmbeddingConversion;

#[derive(Debug, Clone)]
pub struct Client<C, E>
where
    C: CompletionModel,
    E: EmbeddingModel,
{
    completion_model: C,
    embedding_model: E,
}

pub struct RateLimitResponse<T> {
    #[allow(dead_code)]
    response: T,
}

impl
    Client<
        RateLimitedCompletionModel<gemini::completion::CompletionModel>,
        RateLimitedEmbeddingModel<gemini::embedding::EmbeddingModel>,
    >
{
    pub fn new_gemini_from_env() -> Self {
        let gemini_api_key = std::env::var("GEMINI_API_KEY")
            .expect("GEMINI_API_KEY environment variable must be set");
        let gemini_client = gemini::Client::new(&gemini_api_key);
        Self::new_gemini(gemini_client)
    }

    pub fn new_gemini_free_from_env() -> Self {
        let gemini_api_key = std::env::var("GEMINI_FREE_API_KEY")
            .expect("GEMINI_FREE_API_KEY environment variable must be set");
        let gemini_client = gemini::Client::new(&gemini_api_key);
        Self::new_gemini_free(gemini_client)
    }

    pub fn new_gemini(gemini_client: gemini::Client) -> Self {
        let completion_limiter = RateLimiter::direct(Quota::per_minute(
            NonZeroU32::new(2000).expect("must create rate limit"),
        ));
        let embedding_limiter = RateLimiter::direct(Quota::per_minute(
            NonZeroU32::new(1000).expect("must create rate limit"),
        ));
        let completion_model = RateLimitedCompletionModel::new(
            gemini_client.completion_model("gemini-2.0-flash"),
            completion_limiter,
        );
        let embedding_model = RateLimitedEmbeddingModel::new(
            gemini_client.embedding_model(gemini::embedding::EMBEDDING_004),
            embedding_limiter,
        );
        Self {
            completion_model,
            embedding_model,
        }
    }

    pub fn new_gemini_free(gemini_client: gemini::Client) -> Self {
        let completion_limiter = RateLimiter::direct(Quota::per_minute(
            NonZeroU32::new(30).expect("must create rate limit"),
        ));
        let embedding_limiter = RateLimiter::direct(Quota::per_minute(
            NonZeroU32::new(1000).expect("must create rate limit"),
        ));
        let completion_model = RateLimitedCompletionModel::new(
            gemini_client.completion_model("gemini-2.0-flash-lite"),
            completion_limiter,
        );
        let embedding_model = RateLimitedEmbeddingModel::new(
            gemini_client.embedding_model(gemini::embedding::EMBEDDING_004),
            embedding_limiter,
        );
        Self {
            completion_model,
            embedding_model,
        }
    }
}

impl<C, E> Client<C, E>
where
    C: CompletionModel,
    E: EmbeddingModel,
{
    pub fn completion(&self) -> &C {
        &self.completion_model
    }

    pub fn embedding(&self) -> &E {
        &self.embedding_model
    }
}
