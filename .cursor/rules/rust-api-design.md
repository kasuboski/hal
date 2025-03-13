---
title: Rust API Design and Documentation Best Practices
description: Guidelines for creating ergonomic and well-documented Rust APIs
files: ["**/*.rs"]
---

When designing Rust APIs:

- Document everything with rustdoc:
  ```rust
  /// Brief description
  ///
  /// # Examples
  ///
  /// ```rust
  /// let result = my_function(42);
  /// assert_eq!(result, 84);
  /// ```
  ///
  /// # Errors
  ///
  /// Returns `Err` if...
  ///
  /// # Panics
  ///
  /// Panics if...
  pub fn my_function(input: i32) -> Result<i32, MyError>
  ```

- Follow the Rust API Guidelines:
  - Use builder pattern for complex object construction
  - Implement common traits (`Debug`, `Clone`, `PartialEq`) where appropriate
  - Use newtype pattern to provide strong typing
  - Prefer methods over functions when there's a clear receiver

- Type System Best Practices:
  - Use newtypes to enforce invariants at compile time
  - Leverage type state patterns for compile-time guarantees
  - Make illegal states unrepresentable through types

- API Stability:
  - Use semantic versioning
  - Mark unstable features with `#[doc(hidden)]`
  - Use feature flags for experimental features
  - Consider backwards compatibility in public APIs

Remember: A good API is easy to use correctly and hard to use incorrectly. 