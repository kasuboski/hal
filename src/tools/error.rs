use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Command execution failed: {0}")]
    CommandExecutionError(String),
    #[error("Failed to detect shell: {0}")]
    ShellDetectionError(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, thiserror::Error)]
#[error("File operation error: {0}")]
pub struct FileError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("Init error")]
pub struct InitError;
