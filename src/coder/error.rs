//! Error types for the Coder Module.

use rig::completion::CompletionError;
use rig::message::Message;
use rig::tool::ToolSetError;
use thiserror::Error;

/// Errors that can occur during a coder session.
#[derive(Error, Debug)]
pub enum CoderError {
    #[error("Pro agent returned an empty plan")]
    EmptyPlan,

    #[error("Agent provided no initial response")]
    AgentNoInitialResponse,

    #[error("Agent stopped responding")]
    AgentStoppedResponding(Vec<Message>),

    #[error("Junior execution reached maximum iterations ({0})")]
    MaxIterationsReached(usize),

    #[error("Agent completion request failed: {0}")]
    CompletionError(#[from] CompletionError),

    #[error("Tool execution failed: {0}")]
    ToolError(#[from] ToolSetError),

    #[error("Failed to serialize tool arguments: {0}")]
    ToolArgsSerializationError(#[from] serde_json::Error),

    #[error("Agent error: {0}")]
    AgentError(String),

    // Add other specific coder errors as needed
    #[error("Internal coder error: {0}")]
    Internal(String),
}
