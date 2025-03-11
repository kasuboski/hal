//! Error types for the HAL crate

/// Result type for HAL operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for HAL operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// HTTP client error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// API returned an error response
    #[error("API error: {status_code} - {message}")]
    Api {
        /// HTTP status code
        status_code: u16,
        /// Error message
        message: String,
    },

    /// Authentication error
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded. Please retry after {retry_after_secs} seconds")]
    RateLimit {
        /// Seconds to wait before retrying
        retry_after_secs: u64,
    },

    /// Invalid request parameters
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Unexpected response format
    #[error("Unexpected response format: {0}")]
    UnexpectedResponse(String),

    /// Operation not supported
    #[error("Operation not supported: {0}")]
    Unsupported(String),

    /// Other errors
    #[error("{0}")]
    Other(String),
}