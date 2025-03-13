---
title: Rust Performance Optimization Best Practices
description: Guidelines for writing high-performance Rust code
files: ["**/*.rs"]
---

When optimizing Rust code for performance:

- Use `#[inline]` judiciously on small, frequently-called functions
- Prefer stack allocation over heap allocation where possible
- Use `Vec` with pre-allocated capacity when size is known:
  ```rust
  let mut vec = Vec::with_capacity(expected_size);
  ```
- Leverage zero-cost abstractions:
  - Iterator chains instead of explicit loops
  - Generic traits over dynamic dispatch when possible
- Use appropriate data structures:
  - `HashMap` for O(1) lookups
  - `BTreeMap` for sorted keys
  - `SmallVec` for small arrays that might grow
- Profile before optimizing:
  - Use `criterion` for benchmarking
  - Use `flamegraph` for identifying hot spots
- Consider SIMD optimizations using `std::simd` for numeric computations
- Use `rayon` for parallel iterators when processing large data sets
- Minimize allocations in hot paths:
  - Use string interning for repeated strings
  - Reuse allocations where possible
  - Consider arena allocation patterns for complex data structures

Remember: Profile first, optimize later. Premature optimization is the root of all evil. 