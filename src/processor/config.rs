//! Configuration for the processor module

/// Configuration for chunking text
#[derive(Debug, Clone)]
pub struct ChunkOptions {
    /// Target size of each chunk in characters
    pub target_chunk_size: usize,

    /// Size of overlap between chunks in characters
    pub overlap_size: usize,
}

impl Default for ChunkOptions {
    fn default() -> Self {
        Self {
            target_chunk_size: 1000,
            overlap_size: 100,
        }
    }
}

/// Configuration for the processor
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// Options for chunking
    pub chunk_options: ChunkOptions,

    /// LLM model to use for summaries and context
    pub llm_model: String,

    /// Dimensions of the embedding vectors
    pub embedding_dimensions: usize,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            chunk_options: ChunkOptions::default(),
            llm_model: "gemini-1.5-flash".to_string(),
            embedding_dimensions: 384,
        }
    }
}

/// Builder for ProcessorConfig
#[derive(Debug, Default)]
pub struct ProcessorConfigBuilder {
    config: ProcessorConfig,
}

impl ProcessorConfigBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: ProcessorConfig::default(),
        }
    }

    /// Set the chunk options
    pub fn chunk_options(mut self, chunk_options: ChunkOptions) -> Self {
        self.config.chunk_options = chunk_options;
        self
    }

    /// Set the target chunk size
    pub fn target_chunk_size(mut self, target_chunk_size: usize) -> Self {
        self.config.chunk_options.target_chunk_size = target_chunk_size;
        self
    }

    /// Set the overlap size
    pub fn overlap_size(mut self, overlap_size: usize) -> Self {
        self.config.chunk_options.overlap_size = overlap_size;
        self
    }

    /// Set the LLM model
    pub fn llm_model(mut self, llm_model: impl Into<String>) -> Self {
        self.config.llm_model = llm_model.into();
        self
    }

    /// Set the embedding dimensions
    pub fn embedding_dimensions(mut self, embedding_dimensions: usize) -> Self {
        self.config.embedding_dimensions = embedding_dimensions;
        self
    }

    /// Build the configuration
    pub fn build(self) -> ProcessorConfig {
        self.config
    }
}

impl ProcessorConfig {
    /// Create a new builder
    pub fn builder() -> ProcessorConfigBuilder {
        ProcessorConfigBuilder::new()
    }
}
