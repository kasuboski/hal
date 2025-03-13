---
title: Rust Project Structure and Organization
description: Guidelines for organizing Rust projects and managing dependencies
files: ["**/*.rs"]
---

When structuring Rust projects:

- Project Layout:
  ```
  my_project/
  ├── Cargo.toml
  ├── Cargo.lock
  ├── src/
  │   ├── main.rs        # Binary entry point
  │   ├── lib.rs         # Library entry point
  │   ├── bin/           # Additional binaries
  │   ├── auth.rs        # Auth module
  │   ├── auth/          # Auth submodules
  │   │   ├── models.rs
  │   │   └── middleware.rs
  │   ├── api.rs         # API module
  │   ├── api/           # API submodules
  │   │   ├── handlers.rs
  │   │   └── routes.rs
  │   └── tests/         # Integration tests
  ├── examples/          # Example code
  ├── benches/          # Benchmarks
  └── docs/             # Documentation
  ```

- Module Organization:
  - Use modern module naming convention:
    - Create a file with the module name (e.g., `auth.rs`)
    - Create a directory with the same name for submodules (e.g., `auth/`)
    - AVOID using `mod.rs` files (older style)
  ```rust
  // auth.rs - Main module file
  pub mod models;    // Points to auth/models.rs
  pub mod middleware;// Points to auth/middleware.rs
  
  // auth/models.rs - Submodule
  pub struct User { ... }
  ```

- Dependency Management:
  - Use semantic versioning in `Cargo.toml`
  - Minimize dependency count
  - Audit dependencies with `cargo audit`
  ```toml
  [dependencies]
  tokio = { version = "1.0", features = ["full"] }
  serde = { version = "1.0", features = ["derive"] }
  
  [dev-dependencies]
  criterion = "0.5"
  mockall = "0.12"
  ```

- Workspace Organization:
  - Split large projects into workspace members
  - Share common dependencies at workspace level
  - Use internal crates for code sharing
  ```toml
  [workspace]
  members = [
      "core",
      "api",
      "cli",
      "common"
  ]
  ```

- Feature Management:
  - Use feature flags for optional functionality
  - Document feature combinations
  - Consider feature dependencies
  ```toml
  [features]
  default = ["std"]
  std = []
  async = ["tokio", "async-trait"]
  full = ["std", "async"]
  ```

Remember: Good project structure makes code easier to navigate, maintain, and scale. Think about your public API surface carefully. Modern Rust favors explicit module files over `mod.rs` for better discoverability and IDE support. 