//! Error types for the Coder Module.

use rig::completion::CompletionError;
use rig::tool::ToolSetError;
use thiserror::Error;

/// Errors that can occur during a coder session.
#[derive(Error, Debug)]
pub enum CoderError {
    #[error("Pro agent returned an empty plan")]
    EmptyPlan,

    #[error("Junior agent provided no initial response")]
    JuniorNoInitialResponse,

    #[error("Junior agent stopped responding")]
    JuniorStoppedResponding,

    #[error("Junior execution reached maximum iterations ({0})")]
    MaxIterationsReached(usize),

    #[error("Agent completion request failed: {0}")]
    CompletionError(#[from] CompletionError),

    #[error("Tool execution failed: {0}")]
    ToolError(#[from] ToolSetError),

    #[error("Failed to serialize tool arguments: {0}")]
    ToolArgsSerializationError(#[from] serde_json::Error),

    // Add other specific coder errors as needed
    #[error("Internal coder error: {0}")]
    Internal(String),
}
