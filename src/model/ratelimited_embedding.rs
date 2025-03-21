//! # Rate-Limited Embedding Model Module
//!
//! This module provides a wrapper around any embedding model that adds rate limiting
//! capabilities to prevent API quota exhaustion and manage costs when generating
//! embeddings for semantic search.
//!
//! ## Features
//!
//! - Transparent rate limiting for any model implementing the `EmbeddingModel` trait
//! - Configurable rate limits with governor crate integration
//! - Instrumentation with tracing spans for monitoring and debugging
//! - Maintains compatibility with the original model's dimensionality and constraints
//!
//! ## Usage
//!
//! The rate-limited model transparently handles waiting when the rate limit is
//! reached, making it suitable for batch processing of documents where a large
//! number of embeddings need to be generated without exceeding API limits.

use std::sync::Arc;

use governor::DefaultDirectRateLimiter;
use rig::embeddings::{Embedding, EmbeddingError, EmbeddingModel};
use tracing::{debug_span, info_span, Instrument};

#[derive(Clone)]
pub struct RateLimitedEmbeddingModel<M: EmbeddingModel> {
    model: M,
    limiter: Arc<DefaultDirectRateLimiter>,
}

impl<M> RateLimitedEmbeddingModel<M>
where
    M: EmbeddingModel,
{
    pub fn new(model: M, limiter: DefaultDirectRateLimiter) -> Self {
        Self {
            model,
            limiter: Arc::new(limiter),
        }
    }
}

impl<M: EmbeddingModel> EmbeddingModel for RateLimitedEmbeddingModel<M> {
    const MAX_DOCUMENTS: usize = M::MAX_DOCUMENTS;

    fn ndims(&self) -> usize {
        self.model.ndims()
    }

    async fn embed_texts(
        &self,
        texts: impl IntoIterator<Item = String> + Send,
    ) -> Result<Vec<Embedding>, EmbeddingError> {
        self.limiter
            .until_ready()
            .instrument(debug_span!("limiter"))
            .await;
        self.model
            .embed_texts(texts)
            .instrument(info_span!("embed_texts"))
            .await
    }
}
