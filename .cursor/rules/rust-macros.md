---
title: Rust Macro Best Practices
description: Guidelines for writing and using declarative and procedural macros in Rust
files: ["**/*.rs"]
---

When working with Rust macros:

- Declarative Macro Guidelines:
  - Use for simple syntax extensions
  - Follow repetition patterns consistently
  - Handle all edge cases
  ```rust
  macro_rules! vec_of_strings {
      ($($x:expr),* $(,)?) => {
          vec![$($x.to_string()),*]
      };
  }
  ```

- Procedural Macro Types:
  - Function-like: `#[proc_macro]`
  - Derive: `#[proc_macro_derive]`
  - Attribute: `#[proc_macro_attribute]`
  ```rust
  #[proc_macro_derive(MyDerive)]
  pub fn my_derive(input: TokenStream) -> TokenStream {
      let ast = syn::parse(input).unwrap();
      // Generate implementation
  }
  ```

- Error Handling:
  - Use `compile_error!` for declarative macros
  - Use `syn::Error` for procedural macros
  - Provide clear error messages
  ```rust
  macro_rules! assert_type {
      ($x:expr, $t:ty) => {
          if !std::any::type_name::<$t>() == std::any::type_name_of_val(&$x) {
              compile_error!("Type mismatch");
          }
      };
  }
  ```

- Hygiene:
  - Use dollar-crate (`$crate`) for absolute paths
  - Avoid name conflicts with local variables
  - Consider identifier uniqueness
  ```rust
  macro_rules! with_state {
      ($state:expr, $($body:tt)*) => {{
          let $crate::State { mut inner } = $state;
          $($body)*
      }};
  }
  ```

- Documentation:
  - Document macro syntax and usage
  - Provide examples for each pattern
  - Explain expansion behavior
  ```rust
  /// Creates a new widget with the given properties.
  /// 
  /// # Examples
  /// 
  /// ```
  /// widget! {
  ///     title: "My Widget",
  ///     width: 100,
  ///     height: 200
  /// }
  /// ```
  #[macro_export]
  macro_rules! widget { ... }
  ```

Remember: Macros are powerful but complex. Use them judiciously and prioritize readability and maintainability. 