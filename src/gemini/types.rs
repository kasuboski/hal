//! Type definitions for the HAL crate
//!
//! This module contains the core data structures for interacting with the Gemini API.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use base64::Engine;

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
    pub fn with_image_base64(mut self, data: impl Into<String>, mime_type: impl Into<String>) -> Self {
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
#[derive(Debug, Clone)]
pub struct HttpOptions {
    /// API version
    pub api_version: String,
    
    /// Additional HTTP headers
    pub headers: HashMap<String, String>,
}

impl Default for HttpOptions {
    fn default() -> Self {
        Self {
            api_version: "v1beta".to_string(),
            headers: HashMap::new(),
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

/// Tuning dataset for model tuning
#[derive(Debug, Clone, Serialize)]
pub struct TuningDataset {
    /// Examples for tuning
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<TuningExample>>,
    
    /// GCS URI for dataset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcs_uri: Option<String>,
}

/// Example for tuning
#[derive(Debug, Clone, Serialize)]
pub struct TuningExample {
    /// Input text
    pub text_input: String,
    
    /// Output text
    pub output: String,
}

/// Configuration for creating a tuning job
#[derive(Debug, Clone, Serialize)]
pub struct CreateTuningJobConfig {
    /// Number of epochs
    pub epoch_count: i32,
    
    /// Display name for the tuned model
    pub tuned_model_display_name: String,
}

/// Configuration for creating cached content
#[derive(Debug, Clone, Serialize)]
pub struct CreateCachedContentConfig {
    /// Contents to cache
    pub contents: Vec<Content>,
}