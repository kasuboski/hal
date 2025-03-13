---
title: Rust Error Handling Best Practices
description: Guidelines for implementing robust error handling in Rust
files: ["**/*.rs"]
---

When implementing error handling in Rust:

- Use `Result<T, E>` for operations that can fail, avoid using `panic!` or `unwrap()` in production code
- Create custom error types that implement `std::error::Error` for domain-specific errors
- Use the `thiserror` crate for deriving error implementations
- Prefer `?` operator over `.unwrap()` or `.expect()` for error propagation
- When creating custom error types, follow this pattern:
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum MyError {
      #[error("failed to process data: {0}")]
      ProcessError(String),
      #[error("io error: {0}")]
      IoError(#[from] std::io::Error),
  }
  ```
- Use `anyhow::Result<T>` for application code where custom error types are overkill
- Document error conditions in function documentation

Remember: Rust's error handling is explicit and part of the function signature. Make it meaningful and descriptive. 