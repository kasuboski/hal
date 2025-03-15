//! HTTP client implementation for the HAL crate
//!
//! This module provides the HTTP client for making requests to the Gemini API.

use crate::error::{Error, Result};
use crate::gemini::types::HttpOptions;
use rand::{thread_rng, Rng};
use reqwest::{Client as ReqwestClient, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;
use tracing::{debug, error, instrument};
use url::Url;

/// Default timeout for HTTP requests in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// HTTP client for making requests to the Gemini API
///
/// This client handles authentication, request formatting, and response parsing for the Gemini API.
/// It supports both the Gemini Developer API (with API key) and Vertex AI (with project ID and location).
///
/// The client can be configured to automatically retry requests when rate limited (HTTP 429 responses).
/// This behavior is controlled by the `retry_on_rate_limit`, `max_retries`, and `default_retry_after_secs`
/// options in the `HttpOptions` struct.
///
/// # Examples
///
/// ```
/// use hal::gemini::http::HttpClient;
/// use hal::gemini::types::HttpOptions;
///
/// // Create a client with rate limit retry enabled
/// let mut options = HttpOptions::default();
/// options.retry_on_rate_limit = true;
/// let client = HttpClient::with_api_key_and_options("your-api-key".to_string(), options);
/// ```
#[derive(Clone)]
pub struct HttpClient {
    /// The underlying reqwest client
    client: ReqwestClient,

    /// Base URL for API requests
    base_url: String,

    /// API key for authentication (Gemini Developer API)
    api_key: Option<String>,

    /// Project ID for Vertex AI
    project_id: Option<String>,

    /// Location for Vertex AI
    location: Option<String>,

    /// API version
    api_version: String,

    /// Whether to automatically retry requests when rate limited
    retry_on_rate_limit: bool,

    /// Maximum number of retry attempts for rate-limited requests
    max_retries: u32,

    /// Default retry delay in seconds if no Retry-After header is provided
    default_retry_after_secs: u64,

    /// Whether to enable client-side rate limiting
    enable_client_side_rate_limiting: bool,

    /// Maximum number of requests allowed per minute
    requests_per_minute: u32,

    /// Whether to wait when rate limited instead of returning an error
    wait_when_rate_limited: bool,

    /// Last request timestamps for rate limiting (shared across clones)
    #[allow(clippy::type_complexity)]
    request_timestamps:
        std::sync::Arc<tokio::sync::Mutex<std::collections::VecDeque<std::time::Instant>>>,
}

#[cfg(test)]
impl HttpClient {
    /// Set the base URL (for testing only)
    pub fn set_base_url(&mut self, url: String) {
        self.base_url = url;
    }
}

impl HttpClient {
    /// Create a new HTTP client with an API key for the Gemini Developer API
    pub fn with_api_key(api_key: String) -> Self {
        Self::with_api_key_and_options(api_key, HttpOptions::default())
    }

    /// Create a new HTTP client with an API key and custom options
    pub fn with_api_key_and_options(api_key: String, options: HttpOptions) -> Self {
        let client = ReqwestClient::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: "https://generativelanguage.googleapis.com".to_string(),
            api_key: Some(api_key),
            project_id: None,
            location: None,
            api_version: options.api_version,
            retry_on_rate_limit: options.retry_on_rate_limit,
            max_retries: options.max_retries,
            default_retry_after_secs: options.default_retry_after_secs,
            enable_client_side_rate_limiting: options.enable_client_side_rate_limiting,
            requests_per_minute: options.requests_per_minute,
            wait_when_rate_limited: options.wait_when_rate_limited,
            request_timestamps: std::sync::Arc::new(tokio::sync::Mutex::new(
                std::collections::VecDeque::new(),
            )),
        }
    }

    /// Create a new HTTP client for Vertex AI
    pub fn with_vertex_ai(project_id: String, location: String) -> Self {
        Self::with_vertex_ai_and_options(project_id, location, HttpOptions::default())
    }

    /// Create a new HTTP client for Vertex AI with custom options
    pub fn with_vertex_ai_and_options(
        project_id: String,
        location: String,
        options: HttpOptions,
    ) -> Self {
        let client = ReqwestClient::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: format!("https://{}-aiplatform.googleapis.com", location),
            api_key: None,
            project_id: Some(project_id),
            location: Some(location),
            api_version: options.api_version,
            retry_on_rate_limit: options.retry_on_rate_limit,
            max_retries: options.max_retries,
            default_retry_after_secs: options.default_retry_after_secs,
            enable_client_side_rate_limiting: options.enable_client_side_rate_limiting,
            requests_per_minute: options.requests_per_minute,
            wait_when_rate_limited: options.wait_when_rate_limited,
            request_timestamps: std::sync::Arc::new(tokio::sync::Mutex::new(
                std::collections::VecDeque::new(),
            )),
        }
    }

    pub fn reset_default_options(&self) -> Self {
        Self::with_api_key_and_options(self.api_key.clone().unwrap(), HttpOptions::default())
    }

    /// Build a URL for the Gemini Developer API
    fn build_genai_url(&self, path: &str) -> Result<Url> {
        let url = format!("{}/{}/{}", self.base_url, self.api_version, path);
        Url::parse(&url).map_err(|e| Error::Other(format!("Invalid URL: {}", e)))
    }

    /// Build a URL for Vertex AI
    fn build_vertex_url(&self, path: &str) -> Result<Url> {
        if let (Some(project_id), Some(location)) = (&self.project_id, &self.location) {
            let url = format!(
                "{}/{}/projects/{}/locations/{}/{}",
                self.base_url, self.api_version, project_id, location, path
            );
            Url::parse(&url).map_err(|e| Error::Other(format!("Invalid URL: {}", e)))
        } else {
            Err(Error::Auth(
                "Missing project_id or location for Vertex AI".to_string(),
            ))
        }
    }

    /// Prepare a GET request
    #[instrument(skip(self), level = "debug")]
    pub async fn get<T: DeserializeOwned>(&self, path: &str, is_vertex: bool) -> Result<T> {
        let url = if is_vertex {
            self.build_vertex_url(path)?
        } else {
            self.build_genai_url(path)?
        };

        let mut request = self.client.get(url);

        if let Some(api_key) = &self.api_key {
            request = request.query(&[("key", api_key)]);
        }

        debug!("Sending GET request to {}", path);
        self.execute_request(request).await
    }

    /// Prepare a POST request with a JSON body
    #[instrument(skip(self, body), level = "debug")]
    pub async fn post<T: DeserializeOwned, B: Serialize + std::fmt::Debug>(
        &self,
        path: &str,
        body: &B,
        is_vertex: bool,
    ) -> Result<T> {
        let url = if is_vertex {
            self.build_vertex_url(path)?
        } else {
            self.build_genai_url(path)?
        };

        let mut request = self.client.post(url).json(body);

        if let Some(api_key) = &self.api_key {
            request = request.query(&[("key", api_key)]);
        }

        debug!("Sending POST request to {}", path);
        self.execute_request(request).await
    }

    /// Prepare a DELETE request
    #[instrument(skip(self), level = "debug")]
    pub async fn delete<T: DeserializeOwned>(&self, path: &str, is_vertex: bool) -> Result<T> {
        let url = if is_vertex {
            self.build_vertex_url(path)?
        } else {
            self.build_genai_url(path)?
        };

        let mut request = self.client.delete(url);

        if let Some(api_key) = &self.api_key {
            request = request.query(&[("key", api_key)]);
        }

        debug!("Sending DELETE request to {}", path);
        self.execute_request(request).await
    }

    /// Check if a request can be made based on the rate limit
    async fn check_rate_limit(&self) -> Result<()> {
        // Skip rate limiting if disabled
        if !self.enable_client_side_rate_limiting {
            return Ok(());
        }

        loop {
            let now = std::time::Instant::now();
            let window_duration = std::time::Duration::from_secs(60); // 1 minute window

            let mut timestamps = self.request_timestamps.lock().await;

            // Remove timestamps older than the window
            while let Some(timestamp) = timestamps.front() {
                if now.duration_since(*timestamp) > window_duration {
                    timestamps.pop_front();
                } else {
                    break;
                }
            }

            // Check if we're at the rate limit
            if timestamps.len() >= self.requests_per_minute as usize {
                if self.wait_when_rate_limited {
                    // Calculate how long to wait until we can make another request
                    if let Some(oldest) = timestamps.front() {
                        // Calculate time until the oldest timestamp is outside the window
                        let time_until_slot_available = window_duration
                            .checked_sub(now.duration_since(*oldest))
                            .unwrap_or_else(|| std::time::Duration::from_millis(100));

                        // Add a small buffer (10%) to be extra cautious
                        let wait_time = time_until_slot_available.mul_f32(1.1);

                        let current_len = timestamps.len();
                        debug!(
                            "Client-side rate limit reached ({} requests in window). Waiting for {} ms before next request.",
                            current_len,
                            wait_time.as_millis()
                        );

                        // Release the lock while waiting
                        drop(timestamps);
                        tokio::time::sleep(wait_time).await;

                        // Continue the loop and check again
                        continue;
                    }
                } else {
                    // Return an error if we don't want to wait
                    return Err(Error::RateLimit {
                        retry_after_secs: 60, // Suggest waiting for a full window
                    });
                }
            }

            // Add current timestamp and allow the request
            timestamps.push_back(now);

            // If we're getting close to the limit (>80% capacity), add some delay
            // to naturally spread out requests
            let current_len = timestamps.len();
            let requests_ratio = current_len as f32 / self.requests_per_minute as f32;

            if current_len > (self.requests_per_minute as f32 * 0.8) as usize {
                let delay = std::time::Duration::from_millis((requests_ratio * 500.0) as u64);

                // Release the lock before sleeping
                drop(timestamps);

                debug!(
                    "Approaching rate limit ({:.0}% capacity). Adding {} ms delay between requests.",
                    requests_ratio * 100.0,
                    delay.as_millis()
                );

                tokio::time::sleep(delay).await;
            }

            return Ok(());
        }
    }

    /// Execute an HTTP request and handle the response
    async fn execute_request<T: DeserializeOwned>(&self, request: RequestBuilder) -> Result<T> {
        // Apply client-side rate limiting before sending the request
        self.check_rate_limit().await?;

        let mut attempts = 0;

        loop {
            // Clone the request builder for each attempt
            let request_clone = request
                .try_clone()
                .ok_or_else(|| Error::Other("Failed to clone request for retry".to_string()))?;

            let response = request_clone.send().await.map_err(Error::Http)?;
            let status = response.status();

            // Check for rate limit response
            if status == StatusCode::TOO_MANY_REQUESTS {
                attempts += 1;

                // Extract retry-after header if available
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(self.default_retry_after_secs);

                let response_text = response.text().await.map_err(Error::Http)?;
                error!("API error: {} - {}", status, response_text);

                // Check if we should retry
                if self.retry_on_rate_limit && attempts <= self.max_retries {
                    // Calculate backoff with exponential increase and jitter
                    let base_delay = retry_after;
                    let max_delay = 60; // Cap at 60 seconds

                    // Apply exponential backoff: base_delay * 2^(attempts-1) with some jitter
                    let exp_factor = u64::pow(2, attempts - 1);
                    let mut delay = base_delay.saturating_mul(exp_factor);

                    // Add jitter (±20%)
                    if delay > 1 {
                        let jitter_factor = thread_rng().gen_range(0.8..1.2);
                        delay = ((delay as f64) * jitter_factor) as u64;
                    }

                    // Cap at max_delay
                    delay = std::cmp::min(delay, max_delay);

                    debug!(
                        "Rate limited. Retrying after {} seconds (attempt {}/{}, exponential backoff applied)", 
                        delay, attempts, self.max_retries
                    );

                    // Sleep for the calculated duration
                    tokio::time::sleep(Duration::from_secs(delay)).await;

                    // Also update our client-side rate limiter to be more conservative
                    if let Ok(mut timestamps) = self.request_timestamps.try_lock() {
                        // If we got rate limited, our client-side limiter was too aggressive
                        // Add some artificial timestamps to slow down future requests
                        let now = std::time::Instant::now();
                        for i in 0..3 {
                            timestamps.push_back(now - Duration::from_secs(i * 5));
                        }
                    }

                    continue;
                }

                // If we're not retrying or have exceeded max retries, return the error
                return Err(Error::RateLimit {
                    retry_after_secs: retry_after,
                });
            }

            // For non-rate-limit responses, process normally
            let response_text = response.text().await.map_err(Error::Http)?;

            if status.is_success() {
                return serde_json::from_str(&response_text).map_err(|e| {
                    error!("Failed to parse response: {}", e);
                    Error::UnexpectedResponse(format!("Failed to parse response: {}", e))
                });
            } else {
                error!("API error: {} - {}", status, response_text);

                return if status == StatusCode::UNAUTHORIZED {
                    Err(Error::Auth("Invalid API key or credentials".to_string()))
                } else if status == StatusCode::NOT_IMPLEMENTED {
                    Err(Error::Unsupported(format!(
                        "Operation not supported: {}",
                        response_text
                    )))
                } else {
                    Err(Error::Api {
                        status_code: status.as_u16(),
                        message: response_text,
                    })
                };
            }
        }
    }

    /// Create a new HTTP client with an API key and Gemini's default rate limits (30 requests per minute)
    ///
    /// This is a convenience method that creates a client with client-side rate limiting enabled
    /// to stay within Gemini's default limit of 30 requests per minute per model.
    ///
    /// To provide a safety margin, this method sets the client-side limit to 28 requests per minute,
    /// which helps avoid rate limit errors when multiple clients share the same API key or when
    /// there's clock skew between the client and server.
    ///
    /// # Examples
    ///
    /// ```
    /// use hal::gemini::http::HttpClient;
    ///
    /// let client = HttpClient::with_gemini_rate_limits("your-api-key".to_string());
    /// ```
    pub fn with_gemini_rate_limits(api_key: String) -> Self {
        let options = HttpOptions {
            enable_client_side_rate_limiting: true,
            requests_per_minute: 28, // More conservative than Gemini's 30 req/min limit
            wait_when_rate_limited: true,
            retry_on_rate_limit: true,
            max_retries: 5,              // Increase max retries for better resilience
            default_retry_after_secs: 2, // Start with a shorter retry delay
            ..HttpOptions::default()
        };

        Self::with_api_key_and_options(api_key, options)
    }

    /// Create a new HTTP client for Vertex AI with Gemini's default rate limits (30 requests per minute)
    ///
    /// This is a convenience method that creates a client with client-side rate limiting enabled
    /// to stay within Gemini's default limit of 30 requests per minute per model.
    ///
    /// To provide a safety margin, this method sets the client-side limit to 25 requests per minute,
    /// which helps avoid rate limit errors when multiple clients share the same API key or when
    /// there's clock skew between the client and server.
    ///
    /// # Examples
    ///
    /// ```
    /// use hal::gemini::http::HttpClient;
    ///
    /// let client = HttpClient::with_vertex_ai_rate_limits("your-project-id".to_string(), "us-central1".to_string());
    /// ```
    pub fn with_vertex_ai_rate_limits(project_id: String, location: String) -> Self {
        let options = HttpOptions {
            enable_client_side_rate_limiting: true,
            requests_per_minute: 25, // More conservative than Gemini's 30 req/min limit
            wait_when_rate_limited: true,
            retry_on_rate_limit: true,
            max_retries: 5,              // Increase max retries for better resilience
            default_retry_after_secs: 2, // Start with a shorter retry delay
            ..HttpOptions::default()
        };

        Self::with_vertex_ai_and_options(project_id, location, options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct TestResponse {
        message: String,
    }

    #[tokio::test]
    async fn test_get_request_success() {
        let mut server = Server::new_async().await;
        let mock_server = server
            .mock("GET", "/v1beta/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{\"message\": \"success\"}")
            .match_query(mockito::Matcher::Any)
            .expect(1)
            .create_async()
            .await;

        let mut client = HttpClient::with_api_key("test-key".to_string());
        client.set_base_url(server.url());

        let response: TestResponse = client.get("test", false).await.unwrap();
        assert_eq!(response.message, "success");

        mock_server.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_request_success() {
        let mut server = Server::new_async().await;
        let mock_server = server
            .mock("POST", "/v1beta/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{\"message\": \"success\"}")
            .match_query(mockito::Matcher::Any)
            .expect(1)
            .create_async()
            .await;

        let mut client = HttpClient::with_api_key("test-key".to_string());
        client.set_base_url(server.url());

        let body = serde_json::json!({"test": "data"});
        let response: TestResponse = client.post("test", &body, false).await.unwrap();
        assert_eq!(response.message, "success");

        mock_server.assert_async().await;
    }

    #[tokio::test]
    async fn test_error_handling() {
        let mut server = Server::new_async().await;
        let mock_server = server
            .mock("GET", "/v1beta/test")
            .with_status(501)
            .with_body("Not Implemented")
            .match_query(mockito::Matcher::Any)
            .create_async()
            .await;

        let mut client = HttpClient::with_api_key("test-key".to_string());
        client.set_base_url(server.url());

        let result: Result<TestResponse> = client.get("test", false).await;
        assert!(matches!(result, Err(Error::Unsupported(_))));

        mock_server.assert_async().await;
    }

    #[tokio::test]
    async fn test_rate_limit_retry_success() {
        let mut server = Server::new_async().await;

        // First request returns 429 Too Many Requests
        let mock_rate_limit = server.mock("GET", "/v1beta/test")
            .with_status(429)
            .with_header("retry-after", "1")
            .with_body("{\"error\": {\"code\": 429, \"message\": \"Resource has been exhausted\", \"status\": \"RESOURCE_EXHAUSTED\"}}")
            .match_query(mockito::Matcher::Any)
            .expect(1)
            .create_async().await;

        // Second request succeeds
        let mock_success = server
            .mock("GET", "/v1beta/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{\"message\": \"success after retry\"}")
            .match_query(mockito::Matcher::Any)
            .expect(1)
            .create_async()
            .await;

        // Create client with rate limit retry enabled
        let options = HttpOptions {
            retry_on_rate_limit: true,
            default_retry_after_secs: 1, // Use a short delay for testing
            ..HttpOptions::default()
        };

        let mut client = HttpClient::with_api_key_and_options("test-key".to_string(), options);
        client.set_base_url(server.url());

        // Execute request - should retry and succeed
        let response: TestResponse = client.get("test", false).await.unwrap();
        assert_eq!(response.message, "success after retry");

        // Verify both mocks were called
        mock_rate_limit.assert_async().await;
        mock_success.assert_async().await;
    }

    #[tokio::test]
    async fn test_rate_limit_max_retries_exceeded() {
        let mut server = Server::new_async().await;

        // Mock that always returns 429
        let mock_rate_limit = server.mock("GET", "/v1beta/test")
            .with_status(429)
            .with_header("retry-after", "1")
            .with_body("{\"error\": {\"code\": 429, \"message\": \"Resource has been exhausted\", \"status\": \"RESOURCE_EXHAUSTED\"}}")
            .match_query(mockito::Matcher::Any)
            .expect(2) // Expect initial request + 1 retry
            .create_async().await;

        // Create client with rate limit retry enabled but only 1 retry
        let options = HttpOptions {
            retry_on_rate_limit: true,
            max_retries: 1,
            default_retry_after_secs: 1, // Use a short delay for testing
            ..HttpOptions::default()
        };

        let mut client = HttpClient::with_api_key_and_options("test-key".to_string(), options);
        client.set_base_url(server.url());

        // Execute request - should retry once and then fail
        let result: Result<TestResponse> = client.get("test", false).await;
        assert!(matches!(
            result,
            Err(Error::RateLimit {
                retry_after_secs: 1
            })
        ));

        // Verify the mock was called the expected number of times
        mock_rate_limit.assert_async().await;
    }

    #[tokio::test]
    async fn test_client_side_rate_limiting() {
        let options = HttpOptions {
            enable_client_side_rate_limiting: true,
            requests_per_minute: 3,        // Set a low limit for testing
            wait_when_rate_limited: false, // Don't wait, return an error
            ..HttpOptions::default()
        };

        let client = HttpClient::with_api_key_and_options("test-key".to_string(), options);

        // First 3 requests should succeed without rate limiting
        for _ in 0..3 {
            client.check_rate_limit().await.unwrap();
        }

        // The 4th request should be rate limited
        let result = client.check_rate_limit().await;
        assert!(matches!(result, Err(Error::RateLimit { .. })));
    }

    #[tokio::test]
    async fn test_client_side_rate_limiting_with_waiting() {
        let options = HttpOptions {
            enable_client_side_rate_limiting: true,
            requests_per_minute: 3,       // Set a low limit for testing
            wait_when_rate_limited: true, // Wait for a slot to become available
            ..HttpOptions::default()
        };

        let client = HttpClient::with_api_key_and_options("test-key".to_string(), options);

        // First 3 requests should succeed without rate limiting
        for _ in 0..3 {
            client.check_rate_limit().await.unwrap();
        }

        // Modify the timestamps to simulate time passing
        {
            let mut timestamps = client.request_timestamps.lock().await;
            if let Some(timestamp) = timestamps.front_mut() {
                // Make the oldest timestamp 61 seconds old
                *timestamp = std::time::Instant::now() - std::time::Duration::from_secs(61);
            }
        }

        // The 4th request should succeed after "waiting" (we simulated time passing)
        let start = std::time::Instant::now();
        client.check_rate_limit().await.unwrap();

        // The operation should be quick since we manipulated the timestamps
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
    }
}
