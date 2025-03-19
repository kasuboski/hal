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
        self.limiter.until_ready().instrument(debug_span!("limiter")).await;
        self.model.embed_texts(texts).instrument(info_span!("embed_texts")).await
    }
}
