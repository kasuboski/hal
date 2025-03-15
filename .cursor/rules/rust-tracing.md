---
title: Rust Tracing Best Practices
description: Guidelines for effective logging, metrics, and tracing in Rust using the tracing ecosystem
files: ["**/*.rs"]
---

When implementing observability in Rust with tracing:

- Setup and Configuration:
  - Use `tracing` as your primary observability framework
  - Configure subscribers early in your application lifecycle
  - Set up different outputs for development and production
  ```rust
  fn main() {
      // Development setup with pretty console output
      if cfg!(debug_assertions) {
          tracing_subscriber::fmt()
              .with_env_filter("debug")
              .with_target(true)
              .init();
      } else {
          // Production setup with JSON output
          tracing_subscriber::fmt()
              .json()
              .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
              .with_current_span(true)
              .init();
      }
      
      // Application code
  }
  ```

- Structured Logging:
  - Use event macros (`trace!`, `debug!`, `info!`, `warn!`, `error!`)
  - Include relevant context in structured fields
  - Be consistent with log levels across your application
  ```rust
  // Basic event logging
  tracing::info!("Processing request");
  
  // Structured event with fields
  tracing::info!(
      user_id = user.id,
      request_path = %request.path(),
      request_method = %request.method(),
      "Received API request"
  );
  
  // Error logging with context
  let result = process_data(&input);
  if let Err(e) = &result {
      tracing::error!(
          error = %e,
          input_size = input.len(),
          "Failed to process data"
      );
  }
  ```

- Span Management:
  - Create spans to track operations across function boundaries
  - Nest spans to represent hierarchical operations
  - Add relevant context to spans for debugging
  ```rust
  // Function with span
  pub fn process_batch(items: &[Item]) -> Result<(), Error> {
      // Create and enter a span for this function
      let span = tracing::info_span!("process_batch", items_count = items.len());
      let _guard = span.enter();
      
      for (i, item) in items.iter().enumerate() {
          // Create a child span for each item
          let item_span = tracing::debug_span!("process_item", item_id = %item.id, index = i);
          let _item_guard = item_span.enter();
          
          // Process the item
          process_item(item)?;
          
          // The item span is automatically exited when _item_guard is dropped
      }
      
      tracing::info!("Batch processing completed");
      Ok(())
  }
  ```

- Metrics Collection:
  - Use `tracing-opentelemetry` to export metrics
  - Define and record metrics for key operations
  - Track latency, error rates, and throughput
  ```rust
  use metrics::{counter, histogram};
  
  fn handle_request() {
      // Increment request counter
      counter!("api.requests.total", 1);
      
      let start = std::time::Instant::now();
      let result = process_request();
      
      // Record request duration
      let duration = start.elapsed();
      histogram!("api.request.duration", duration);
      
      // Track errors
      if result.is_err() {
          counter!("api.requests.errors", 1);
      }
  }
  ```

- Distributed Tracing:
  - Propagate context across service boundaries
  - Use `tracing-opentelemetry` to integrate with OpenTelemetry
  - Ensure consistent trace IDs across your system
  ```rust
  // Configure OpenTelemetry
  fn init_tracing() {
      let tracer = opentelemetry_jaeger::new_pipeline()
          .with_service_name("my-service")
          .install_simple()
          .expect("Failed to install OpenTelemetry tracer");
          
      tracing_subscriber::registry()
          .with(tracing_subscriber::EnvFilter::from_default_env())
          .with(tracing_opentelemetry::layer().with_tracer(tracer))
          .init();
  }
  
  // Propagate context in HTTP clients
  async fn call_service(client: &Client, url: &str, trace_context: &HeaderMap) -> Result<Response, Error> {
      let mut request = Request::new(Method::GET, url.parse()?);
      
      // Inject trace context into headers
      opentelemetry::global::get_text_map_propagator(|propagator| {
          propagator.inject_context(
              &tracing::Span::current().context(),
              &mut opentelemetry_http::HeaderInjector(request.headers_mut())
          )
      });
      
      client.execute(request).await
  }
  ```

Remember: Good observability is key to understanding your system's behavior in production. Invest time in setting up proper tracing early in your project. 