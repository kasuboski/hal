---
title: Rust Unsafe and FFI Best Practices
description: Guidelines for writing safe unsafe code and FFI bindings in Rust
files: ["**/*.rs"]
---

When working with unsafe code and FFI:

- Unsafe Code Guidelines:
  - Minimize unsafe block size to smallest possible scope
  - Document all safety invariants that must be upheld
  - Wrap unsafe code in safe abstractions
  ```rust
  /// Safety: The pointer must be properly aligned and must point to initialized data
  pub unsafe fn raw_operation(ptr: *const u8) -> Result<u8, Error> {
      // Minimal unsafe block
      let value = unsafe { *ptr };
      process_value(value)
  }
  ```

- FFI Best Practices:
  - Use `#[repr(C)]` for FFI-compatible structs
  - Handle null pointers and invalid UTF-8
  - Prevent undefined behavior at FFI boundaries
  ```rust
  #[repr(C)]
  pub struct FFIStruct {
      data: *mut c_void,
      len: size_t,
  }
  
  #[no_mangle]
  pub extern "C" fn rust_function(input: *const c_char) -> FFIStruct {
      let c_str = unsafe {
          if input.is_null() {
              return FFIStruct::null();
          }
          CStr::from_ptr(input)
      };
      // Process safely converted data
  }
  ```

- Memory Safety:
  - Implement `Drop` for proper cleanup
  - Use RAII patterns for resource management
  - Validate all external inputs
  ```rust
  pub struct SafeWrapper {
      raw: *mut RawType,
  }
  
  impl Drop for SafeWrapper {
      fn drop(&mut self) {
          unsafe {
              if !self.raw.is_null() {
                  free_resource(self.raw);
              }
          }
      }
  }
  ```

- Common Unsafe Patterns:
  - Custom allocators and memory management
  - Platform-specific optimizations
  - Hardware interface code
  ```rust
  #[repr(transparent)]
  pub struct AlignedBuffer {
      data: *mut u8,
  }
  
  impl AlignedBuffer {
      pub fn new(size: usize, align: usize) -> Self {
          unsafe {
              let layout = Layout::from_size_align_unchecked(size, align);
              let ptr = alloc(layout);
              Self { data: ptr }
          }
      }
  }
  ```

Remember: Unsafe code requires extreme care and thorough documentation. Every unsafe block is a contract with the rest of your code. 