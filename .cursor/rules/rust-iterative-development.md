---
title: Rust Iterative Development
description: Guidelines for effective iterative development in Rust
files: ["**/*.rs"]
---

When developing iteratively in Rust:

- Small, Incremental Changes:
  - Make small, focused changes to one component at a time
  - Commit frequently with descriptive messages
  - Refactor code in separate steps from adding new functionality
  ```rust
  // Before: Complex function with multiple responsibilities
  fn process_data(data: &[u8]) -> Result<Output, Error> {
      // 50+ lines of code doing many things
  }
  
  // After: Split into smaller, focused functions
  fn process_data(data: &[u8]) -> Result<Output, Error> {
      let validated = validate_input(data)?;
      let processed = transform_data(validated)?;
      finalize_output(processed)
  }
  ```

- Continuous Compilation:
  - Run `cargo check` after each meaningful change
  - Use `cargo clippy` to catch common mistakes and anti-patterns
  - Use `cargo fmt` to align code formatting to recommendations
  - Set up environment before running cargo commands:
  ```bash
  # Always reload environment before running cargo
  direnv reload && cargo check
  
  # Fix linting issues automatically
  direnv reload && cargo clippy --fix --allow-dirty
  ```

- Effective Error Handling:
  - Use the type system to prevent errors at compile time
  - Implement custom error types incrementally
  - Add context to errors with `anyhow` or `thiserror`
  ```rust
  // Start simple
  fn my_function() -> Result<(), Box<dyn std::error::Error>> {
      // Implementation
  }
  
  // Evolve to custom errors
  #[derive(Debug, thiserror::Error)]
  enum MyError {
      #[error("invalid input: {0}")]
      InvalidInput(String),
      #[error("processing failed: {0}")]
      ProcessingFailed(String),
  }
  
  fn my_function() -> Result<(), MyError> {
      // Implementation
  }
  ```

- Iterative API Design:
  - Start with concrete implementations
  - Extract interfaces (traits) as patterns emerge
  - Use feature flags for experimental features
  ```rust
  // Initial implementation
  struct MyService {
      // fields
  }
  
  impl MyService {
      pub fn process(&self, input: Input) -> Result<Output, Error> {
          // Implementation
      }
  }
  
  // Later: Extract trait when patterns stabilize
  trait Service {
      fn process(&self, input: Input) -> Result<Output, Error>;
  }
  
  impl Service for MyService {
      fn process(&self, input: Input) -> Result<Output, Error> {
          // Implementation
      }
  }
  ```

- Debugging Workflow:
  - Use `dbg!` or `println!` for quick debugging
  - Leverage `#[derive(Debug)]` for better debug output
  - Add tracing with different verbosity levels
  ```rust
  // Quick debugging
  let result = dbg!(complex_calculation(input));
  
  // Structured tracing
  tracing::debug!(input = ?input, "Processing input");
  let result = complex_calculation(input);
  tracing::info!(result = ?result, "Calculation result");
  
  // Span-based debugging
  let span = tracing::info_span!("calculation", input_size = input.len());
  let _guard = span.enter();
  let result = complex_calculation(input);
  tracing::info!("Calculation completed");
  ```

Remember: Compile early and often. Each successful compilation is a checkpoint in your development process. The Rust compiler is your ally in building robust software. 