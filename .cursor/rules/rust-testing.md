---
title: Rust Testing Best Practices
description: Guidelines for writing effective tests in Rust
files: ["**/*.rs"]
---

When writing tests in Rust:

- Test Organization:
  - Use `#[cfg(test)]` module in the same file as the code being tested
  - Create separate integration tests in `tests/` directory
  - Use `proptest` or `quickcheck` for property-based testing
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      
      #[test]
      fn test_functionality() {
          // Unit test here
      }
      
      #[test]
      fn test_error_conditions() {
          // Error case testing
      }
  }
  ```

- Test Structure:
  - Follow Arrange-Act-Assert pattern
  - Use descriptive test names that explain the scenario
  - Test both success and failure cases
  - Use test fixtures for complex setup:
  ```rust
  #[fixture]
  fn test_data() -> TestStruct {
      TestStruct::builder()
          .field1("test")
          .field2(42)
          .build()
          .unwrap()
  }
  ```

- Mock and Stub:
  - Use trait objects for dependency injection
  - Implement `mockall` for mocking complex behaviors
  - Create test-specific trait implementations
  ```rust
  #[cfg_attr(test, mockall::automock)]
  trait Database {
      async fn query(&self, id: &str) -> Result<Data, Error>;
  }
  ```

- Test Coverage:
  - Use `cargo tarpaulin` for coverage reporting
  - Aim for high coverage of business logic
  - Test edge cases and error conditions
  - Include doc tests for API examples

- Performance Testing:
  - Use `criterion` for benchmarking
  - Test with realistic data sizes
  - Include stress tests for concurrent operations
  ```rust
  use criterion::{criterion_group, criterion_main, Criterion};
  
  fn benchmark(c: &mut Criterion) {
      c.bench_function("my_function", |b| {
          b.iter(|| my_function())
      });
  }
  ```

Remember: Tests should be maintainable, readable, and reliable. They are part of your codebase's documentation. 