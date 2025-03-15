//! Type definitions for the HAL crate
//!
//! This module contains the core data structures for interacting with the Gemini API.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Content represents a piece of content that can be processed by the model.
/// It can contain text, images, or other media types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// The role of the content (e.g., "user", "model")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// The parts that make up this content
    pub parts: Vec<Part>,
}

impl Default for Content {
    fn default() -> Self {
        Self::new()
    }
}

impl Content {
    /// Create a new empty content
    pub fn new() -> Self {
        Self {
            role: None,
            parts: Vec::new(),
        }
    }

    /// Set the role for this content
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    /// Add text to this content
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.parts.push(Part::Text(text.into()));
        self
    }

    /// Add an image to this content from base64-encoded data
    pub fn with_image_base64(
        mut self,
        data: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        self.parts.push(Part::Image(Image {
            mime_type: mime_type.into(),
            data: ImageData::Base64(data.into()),
        }));
        self
    }
}

/// A part of content, which can be text, an image, or other media
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Part {
    /// Text content
    #[serde(rename = "text")]
    Text(String),

    /// Image content
    #[serde(rename = "inline_data")]
    Image(Image),

    /// File reference
    #[serde(rename = "file_data")]
    FileData(FileData),
}

impl Part {
    /// Create a part from bytes with a specified MIME type
    pub fn from_bytes(data: Vec<u8>, mime_type: impl Into<String>) -> Self {
        let base64_data = base64::engine::general_purpose::STANDARD.encode(data);
        Part::Image(Image {
            mime_type: mime_type.into(),
            data: ImageData::Base64(base64_data),
        })
    }
}

/// Image data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    /// MIME type of the image
    pub mime_type: String,

    /// The image data
    #[serde(flatten)]
    pub data: ImageData,
}

/// Image data can be provided in different formats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImageData {
    /// Base64-encoded image data
    Base64(String),

    /// URL to an image
    Url(String),
}

/// File data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileData {
    /// File URI
    pub file_uri: String,

    /// MIME type of the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Generation configuration for content generation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerationConfig {
    /// Temperature controls randomness in generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Top-k controls diversity by limiting to k most likely tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,

    /// Top-p controls diversity by limiting to tokens with cumulative probability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Candidate count for multiple generations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<i32>,

    /// Maximum output tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,

    /// Stop sequences to end generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

/// Safety settings for content generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySetting {
    /// The harm category
    pub category: HarmCategory,

    /// The threshold level
    pub threshold: HarmBlockThreshold,
}

/// Categories of potential harm
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmCategory {
    /// Harassment content
    Harassment,
    /// Hate speech
    HateSpeech,
    /// Sexually explicit content
    SexuallyExplicit,
    /// Dangerous content
    Dangerous,
}

/// Thresholds for blocking content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmBlockThreshold {
    /// Block only high harm content
    BlockOnlyHigh,
    /// Block medium and high harm content
    BlockMediumAndHigh,
    /// Block low, medium, and high harm content
    BlockLowAndAbove,
    /// Block very low, low, medium, and high harm content
    BlockNone,
}

/// Response from content generation
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateContentResponse {
    /// The generated candidates
    pub candidates: Vec<Candidate>,

    /// Prompt feedback
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_feedback: Option<PromptFeedback>,
}

impl GenerateContentResponse {
    /// Get the text from the first candidate's first part
    pub fn text(&self) -> String {
        if let Some(candidate) = self.candidates.first() {
            if let Some(content) = candidate.content.as_ref() {
                for part in &content.parts {
                    if let Part::Text(text) = part {
                        return text.clone();
                    }
                }
            }
        }
        String::new()
    }
}

/// A candidate response from the model
#[derive(Debug, Clone, Deserialize)]
pub struct Candidate {
    /// The content of the candidate
    pub content: Option<Content>,

    /// Finish reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,

    /// Safety ratings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<SafetyRating>>,
}

/// Safety rating for generated content
#[derive(Debug, Clone, Deserialize)]
pub struct SafetyRating {
    /// The harm category
    pub category: HarmCategory,

    /// Probability of harm
    pub probability: HarmProbability,
}

/// Probability levels for harm
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmProbability {
    /// Negligible probability
    Negligible,
    /// Low probability
    Low,
    /// Medium probability
    Medium,
    /// High probability
    High,
}

/// Feedback on the prompt
#[derive(Debug, Clone, Deserialize)]
pub struct PromptFeedback {
    /// Safety ratings for the prompt
    pub safety_ratings: Vec<SafetyRating>,

    /// Whether the prompt was blocked
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<String>,
}

/// HTTP options for client configuration
///
/// This struct allows customizing the behavior of the HTTP client, including:
/// - API version to use
/// - Custom HTTP headers
/// - Rate limit retry behavior
/// - Client-side rate limiting to stay within API quotas
///
/// # Examples
///
/// ```
/// use hal::gemini::types::HttpOptions;
///
/// // Create options with rate limit retry enabled
/// let mut options = HttpOptions::default();
/// options.retry_on_rate_limit = true;
/// options.max_retries = 5;
/// options.default_retry_after_secs = 30;
///
/// // Enable client-side rate limiting for Gemini's 30 requests per minute limit
/// options.enable_client_side_rate_limiting = true;
/// options.requests_per_minute = 30;
/// ```
#[derive(Debug, Clone)]
pub struct HttpOptions {
    /// API version
    pub api_version: String,

    /// Additional HTTP headers
    pub headers: HashMap<String, String>,

    /// Whether to automatically retry requests when rate limited
    /// When set to true, the client will automatically retry requests that receive a 429 Too Many Requests response.
    pub retry_on_rate_limit: bool,

    /// Maximum number of retry attempts for rate-limited requests
    /// The client will retry up to this many times before giving up and returning an error.
    pub max_retries: u32,

    /// Default retry delay in seconds if no Retry-After header is provided
    /// This value is used when the server doesn't specify a Retry-After header.
    pub default_retry_after_secs: u64,

    /// Whether to enable client-side rate limiting to prevent 429 responses
    /// When enabled, the client will automatically limit the request rate to stay within the API's limits.
    pub enable_client_side_rate_limiting: bool,

    /// Maximum number of requests allowed per minute
    /// For Gemini API, the default limit is 30 requests per minute per model.
    pub requests_per_minute: u32,

    /// Whether to wait when rate limited instead of returning an error
    /// When true, the client will wait until a token is available rather than returning an error.
    pub wait_when_rate_limited: bool,
}

impl Default for HttpOptions {
    fn default() -> Self {
        Self {
            api_version: "v1beta".to_string(),
            headers: HashMap::new(),
            retry_on_rate_limit: false,
            max_retries: 3,
            default_retry_after_secs: 60,
            enable_client_side_rate_limiting: false,
            requests_per_minute: 30, // Gemini API default limit
            wait_when_rate_limited: true,
        }
    }
}

/// Token count response
#[derive(Debug, Clone, Deserialize)]
pub struct CountTokensResponse {
    /// Total tokens counted
    pub total_tokens: i32,
}

/// Embedding response
#[derive(Debug, Clone, Deserialize)]
pub struct EmbedContentResponse {
    /// The generated embeddings
    pub embedding: Embedding,
}

/// Embedding data
#[derive(Debug, Clone, Deserialize)]
pub struct Embedding {
    /// The embedding values
    pub values: Vec<f32>,
}

/// Configuration for creating cached content
#[derive(Debug, Clone, Serialize)]
pub struct CreateCachedContentConfig {
    /// Contents to cache
    pub contents: Vec<Content>,
}
