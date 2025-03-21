//! # Processor Error Types Module
//! 
//! This module defines error types specific to the content processor component of the RAG pipeline.
//! It provides structured error handling for various failure modes during content processing.
//! 
//! ## Key Components
//! 
//! - `ProcessError`: Enum representing different types of processor failures
//! 
//! ## Features
//! 
//! - Specialized error types for different processor failure scenarios
//! - Embedding-related errors for vector generation issues
//! - LLM-related errors for summarization and context generation failures
//! - Chunking errors for text segmentation problems
//! - Task and concurrency management errors
//! - Detailed error messages for easier debugging
//! 
//! The error handling in this module ensures that failures in the content processing
//! pipeline are properly captured, reported, and can be appropriately handled or
//! recovered from in the broader RAG system.

use rig::embeddings::EmbeddingError;
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

    #[error("Embedding error: {0}")]
    Embedding(#[from] EmbeddingError),

    #[error("Embedding processing error: {0}")]
    EmbeddingProcessing(String),

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
