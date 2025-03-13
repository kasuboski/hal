---
title: Rust Async Programming Best Practices
description: Guidelines for writing efficient and maintainable async Rust code
files: ["**/*.rs"]
---

When writing async Rust code:

- Runtime Selection:
  - Use `tokio` for production-grade server applications
  - Use `async-std` for simpler async scenarios
  - Consider `smol` for minimal runtime overhead
  - Always document which async runtime your code requires

- Task Management:
  - Spawn CPU-intensive tasks on dedicated threads using `spawn_blocking`
  - Use `select!` for handling multiple concurrent operations
  - Implement graceful shutdown with cancellation points
  ```rust
  tokio::select! {
      _ = async_operation() => { /* Handle completion */ }
      _ = shutdown_signal => { /* Handle shutdown */ }
  }
  ```

- Error Handling:
  - Propagate errors with `async fn foo() -> Result<T, Error>`
  - Use `.await?` for error propagation in async contexts
  - Handle task panics with `JoinHandle::catch_unwind()`

- Performance:
  - Avoid blocking operations in async contexts
  - Use connection/object pools for expensive resources
  - Implement backpressure for stream processing
  ```rust
  use futures::stream::StreamExt;
  
  async fn process_stream(mut stream: impl Stream<Item = T>) {
      stream
          .filter_map(|item| async { /* process item */ })
          .buffer_unordered(10) // Concurrent but limited
          .for_each(|result| async { /* handle result */ })
          .await;
  }
  ```

- Testing:
  - Use `#[tokio::test]` for async test functions
  - Mock time with `tokio::time::pause()`
  - Test timeout scenarios with `tokio::time::timeout`
  ```rust
  #[tokio::test]
  async fn test_with_timeout() {
      tokio::time::timeout(
          Duration::from_secs(1),
          async_operation()
      ).await.expect("operation timed out");
  }
  ```

- Common Pitfalls to Avoid:
  - Don't hold `Mutex` locks across `.await` points
  - Avoid long-running CPU tasks in async contexts
  - Don't create infinite loops without yield points
  - Be cautious with `async` in trait implementations

Remember: Async Rust is about managing resources efficiently, not just about concurrent execution. Design for composability and resource management. 