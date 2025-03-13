---
title: Rust Ownership and Borrowing Best Practices
description: Guidelines for proper memory management and ownership patterns in Rust
files: ["**/*.rs"]
---

When working with Rust's ownership system:

- Prefer borrowing (`&T` or `&mut T`) over taking ownership when possible
- Use `Clone` only when necessary and document why it's needed
- Implement `Copy` trait only for small, stack-based types (generally <= 128 bytes)
- Follow the single-writer XOR multiple-readers rule:
  - One mutable reference (`&mut T`) XOR
  - Any number of immutable references (`&T`)
- Use lifetimes explicitly when the compiler needs help:
  ```rust
  fn process<'a>(data: &'a str, other: &'a str) -> &'a str
  ```
- Consider using `Cow<'a, T>` when you need owned data only sometimes
- Use `Arc<T>` for shared ownership across threads, `Rc<T>` for single-threaded cases
- When using interior mutability:
  - Prefer `RefCell<T>` for single-threaded scenarios
  - Use `Mutex<T>` or `RwLock<T>` for thread-safe mutable access

Remember: The borrow checker is your friend, not your enemy. Design your data structures around ownership. 