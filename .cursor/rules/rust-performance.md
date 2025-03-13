---
title: Rust Performance Optimization Best Practices
description: Guidelines for writing high-performance Rust code
files: ["**/*.rs"]
---

When optimizing Rust code for performance:

- Compile-Time Optimizations:
  - Use appropriate optimization levels in Cargo.toml
  - Enable Link-Time Optimization (LTO) for releases
  - Configure codegen units for better optimization
  ```toml
  [profile.release]
  opt-level = 3
  lto = "thin"      # or "fat" for maximum optimization
  codegen-units = 1 # Maximum optimization, slower compilation
  debug = false     # Remove debug symbols
  ```

- Memory Management:
  - Prefer stack allocation over heap allocation
  - Use `Vec` with pre-allocated capacity when size is known
  - Implement custom allocators for specific use cases
  ```rust
  // Pre-allocation
  let mut vec = Vec::with_capacity(expected_size);
  
  // Custom allocator
  #[global_allocator]
  static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;
  
  // Arena allocation for temporary allocations
  use bumpalo::Bump;
  let arena = Bump::new();
  let temp_data = arena.alloc_slice_copy(&[1, 2, 3]);
  ```

- Data Structure Selection:
  - Use appropriate collections for the use case:
    - `HashMap` for O(1) lookups
    - `BTreeMap` for sorted keys
    - `SmallVec` for small arrays that might grow
  - Consider specialized data structures:
    - `indexmap` for ordered maps
    - `dashmap` for concurrent access
    - `tinyvec` for stack-based small vectors

- Zero-Cost Abstractions:
  - Use iterator chains instead of explicit loops
  - Prefer generic traits over dynamic dispatch
  - Leverage const generics for compile-time optimization
  ```rust
  // Iterator chains
  let sum: i32 = items
      .iter()
      .filter(|x| x.is_valid())
      .map(|x| x.value)
      .sum();
  
  // Const generics
  pub struct Buffer<const N: usize> {
      data: [u8; N]
  }
  ```

- Concurrency Optimization:
  - Use `rayon` for parallel iterators
  - Implement SIMD operations with `std::simd`
  - Choose appropriate synchronization primitives
  ```rust
  use rayon::prelude::*;
  
  // Parallel processing
  let result: Vec<_> = data
      .par_iter()
      .filter_map(process_item)
      .collect();
  
  // SIMD operations
  #[cfg(target_arch = "x86_64")]
  use std::arch::x86_64::*;
  ```

- Profile-Guided Optimization:
  - Use `criterion` for benchmarking
  - Profile with `flamegraph` for hotspots
  - Enable PGO in release builds
  ```toml
  [profile.release]
  debug = true  # Keep symbols for profiling
  
  [profile.bench]
  lto = true
  codegen-units = 1
  opt-level = 3
  ```

- Memory Access Patterns:
  - Align data structures for cache efficiency
  - Use cache-friendly traversal patterns
  - Minimize pointer chasing
  ```rust
  #[repr(align(64))]  // Cache line alignment
  struct CacheAligned {
      data: [u8; 64]
  }
  
  // Cache-friendly traversal
  for chunk in data.chunks(64) {
      process_aligned_data(chunk);
  }
  ```

Remember: Profile first, optimize later. Premature optimization is the root of all evil. Always measure the impact of optimizations. 