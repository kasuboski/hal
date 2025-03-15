//! Error types for the processor module

use crate::error::Error as CrateError;
use thiserror::Error;

/// Error type for processor operations
#[derive(Debug, Error)]
pub enum ProcessError {
    /// HTTP client error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Markdown parsing error
    #[error("Markdown parsing error: {0}")]
    MarkdownParse(String),

    /// Embedding generation error
    #[error("Embedding generation error: {0}")]
    EmbeddingGeneration(String),

    /// LLM error
    #[error("LLM error: {0}")]
    Llm(String),

    /// Chunking error
    #[error("Chunking error: {0}")]
    Chunking(String),

    /// Error during semaphore acquisition
    #[error("Semaphore acquisition error: {0}")]
    Semaphore(String),

    /// Error during task joining
    #[error("Task join error: {0}")]
    TaskJoin(String),

    /// Task execution error
    #[error("Task execution error: {0}")]
    Task(String),

    /// Other errors
    #[error("{0}")]
    Other(String),
}

impl From<ProcessError> for CrateError {
    fn from(err: ProcessError) -> Self {
        match err {
            ProcessError::Http(e) => CrateError::Http(e),
            _ => CrateError::Process(err.to_string()),
        }
    }
}

impl From<tokio::sync::AcquireError> for ProcessError {
    fn from(err: tokio::sync::AcquireError) -> Self {
        Self::Semaphore(format!("Failed to acquire semaphore: {}", err))
    }
}

impl From<tokio::task::JoinError> for ProcessError {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::TaskJoin(format!("Failed to join task: {}", err))
    }
}
