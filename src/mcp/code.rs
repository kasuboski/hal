//! This module provides functionalities for various code-related tools.
//!
//! It includes features such as:
//!
//! - Repository overview
//! - Code search
//! - Code retrieval (get)
//! - ... (and potentially other code tool functions)
//!

use yek::config::YekConfig;

/// Generates an overview of the repository.
/// The output is a tuple of the overview and a list of files.
pub fn overview(
    config: &YekConfig,
) -> Result<(String, Vec<yek::parallel::ProcessedFile>), CodeError> {
    yek::serialize_repo(config).map_err(|e| CodeError::SerializeRepoError(e.to_string()))
}

#[derive(thiserror::Error, Debug)]
pub enum CodeError {
    #[error("Serialize repo error: {0}")]
    SerializeRepoError(String),
}
