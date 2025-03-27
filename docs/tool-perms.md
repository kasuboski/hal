Okay, let's rewrite the plan with the explicit requirement that the user passes in a state struct to hold permissions and other dependencies. This approach emphasizes external state management rather than internal, making the tools more flexible and testable.

**1. Understanding the Existing MCP Permission and State System (unchanged):**

*   **`src/mcp/permissions.rs`:**  (Same as before)
*   **`src/mcp/tools.rs`:** (Same as before, but now viewed as a *reference* for the desired behavior)
*   **`src/mcp/tools.rs` (State):** (Same as before, but the *concept* of a state is what's important, not the specific implementation)
*   **`src/mcp/executor.rs` and `src/mcp/shell_utils.rs`:** (Same as before)

**2. Analyzing the `src/tools` Implementation (unchanged):**

*   **`src/tools/project.rs`:** (Same as before)
*   **`src/tools/file.rs`:** (Same as before)
*   **`src/tools/<tool_name>.rs`:** (Same as before)

**3. Defining the Scope and Goals (revised):**

*   The goal is to integrate a user-provided `State` struct (containing, at a minimum, a `SessionPermissions` instance) into the `src/tools` implementation. The tools will *require* this `State` to function, making permission checks mandatory.
*   This will involve:
    *   Defining a `State` trait that the user must implement.
    *   Modifying tool definitions to accept a `State` trait object.
    *   Modifying the tool handlers to use the `State` trait object for permission checks and other dependencies.

**4. Implementation Plan (revised):**

*   **Step 1: Define a `State` Trait in `src/tools/project.rs`:**
```rust
pub trait State: Send + Sync {
    fn permissions(&self) -> &Arc<Mutex<SessionPermissions>>;
}
```

    *   This trait *requires* the user-provided state to expose a `permissions` method that returns a reference to the `PermissionsRef` (which, as before, is `Arc<Mutex<SessionPermissions>>`). The `Send + Sync` bounds are crucial for thread safety.  Other methods can be added to the `State` trait as needed to expose other dependencies.
*   **Step 2: Copy the `SessionPermissions` struct and related functions from `src/mcp/permissions.rs` to `src/tools/project.rs`.** (Same as before)
*   **Step 3: Modify the `Tool` Trait to Accept a `State` Trait Object:**

```rust
use rig::tool::Tool;
use anyhow::Result;

pub trait Tool: Send + Sync {
    const NAME: &'static str;
    type Error: std::error::Error + Send + Sync + 'static;
    type Args: Send + Sync;
    type Output: Send + Sync;

    async fn definition(&self, prompt: String) -> rig::completion::ToolDefinition;
    async fn call(&self, args: Self::Args, state: &dyn State) -> Result<Self::Output, Self::Error>;
}
```
    *   The critical change is adding `state: &dyn State` to the `call` function signature. This forces each tool to accept a reference to a trait object implementing the `State` trait.  This is how the user provides the tool with its dependencies.
*   **Step 4: Implement the `ToolEmbedding` trait for all tools to validate the presence of the State.**
    *   Enforce that the tool's init function has access to the state.
*   **Step 5: Modify the file operation tools in `src/tools/file.rs` to check permissions using the `State` object before performing any file operations.** (Similar to before, but now using the trait object)

```rust
// In src/tools/file.rs

impl Tool for ShowFile {
    // ...

    async fn call(&self, args: Self::Args, state: &dyn State) -> Result<Self::Output, Self::Error> {
        let path = PathBuf::from(&args.path);

        // Check read permission
        let perms = state.permissions().lock().await;
        if !perms.can_read(&path) {
            return Err(FileError(format!(
                "Read permission not granted for path: {}. Request permission first.",
                path.display()
            )));
        }

        // ... rest of the code
    }
}
```

*   **Step 6: Modify the `ExecuteShellCommand` tool in `src/tools/file.rs` to check permissions before executing any commands using the `State` object.** (Similar to before)
*   **Step 7: Modify the `RequestPermission` tool in `src/tools/project.rs` to update the `SessionPermissions` through the `State` object.** (Similar to before)


```rust
// In src/tools/project.rs

impl Tool for RequestPermission {
    // ...

    async fn call(&self, args: Self::Args, state: &dyn State) -> Result<Self::Output, Self::Error> {
        let path_buf = PathBuf::from(&args.path);

        let mut perms = state.permissions().lock().await;
        match args.operation.as_str() {
            "read" => {
                perms.allow_read(dir_path.clone());
            }
            "write" => {
                perms.allow_write(dir_path.clone());
            }
            "execute" => {
                perms.allow_command(program.to_string());
            }
            _ => return Err(FileError(format!("Unknown operation: {}", args.operation))),
        };

        // ... rest of the code
    }
}
```

*   **Step 8:  Modify the `Init` tool in `src/tools/project.rs` to initialize the permissions within the `State` object that's passed in.** The `Init` tool should *require* a mutable state object to set initial permissions. This makes it clear that initialization *mutates* the state.

```rust
// In src/tools/project.rs

impl Tool for Init {
    // ...

    async fn call(&self, args: Self::Args, state: &dyn State) -> Result<Self::Output, Self::Error> {
        let path_buf = PathBuf::from(&args.path);

        // You will need to cast the &dyn State to a concrete type to modify it
        // This is one of the limitations of this pattern. You will need to ensure
        // that the concrete type implements the State trait.
        let mut state =  state.to_mutable_state()?; // to_mutable_state is a method you define

        let mut perms = state.permissions().lock().await;
        perms.allow_read(dir_path.clone());
        perms.allow_write(dir_path.clone());

        // ... rest of the code
    }
}
```

    *   This introduces a key design decision: How does the `Init` tool *mutate* the state if it only receives a `&dyn State` (an immutable reference)? There are several options:
        1.  **Introduce a `to_mutable_state()` method on the `State` trait:** This method would return a `Result<Box<dyn MutableState>, Self::Error>`, where `MutableState` is another trait that allows mutable access to the permissions. This is the most flexible approach but adds complexity.
        2.  **Require a specific concrete type for the `State`:**  The `Init` tool could require that the `State` be a specific struct type that it knows how to mutate. This is simpler but less flexible.
        3.  **Use interior mutability within the `State` implementation:** The `State` implementation could use `RefCell` or `Mutex` internally to allow mutation even through a shared reference. This is generally discouraged as it can lead to runtime errors.
    *   The example code uses option 1, the `to_mutable_state()` method, as it's the most flexible. You'll need to define the `MutableState` trait and implement it for your concrete state type.

*   **Step 9: Update the tool definitions to reflect the new `State` requirements.** (Mostly unchanged)
*   **Step 10: Test the changes thoroughly.** (Unchanged)

**5. Code Modification Examples:**

*   (See examples in the previous plan, but remember to access the `PermissionsRef` through the `State` trait object: `state.permissions()`)

**6. Considerations (revised):**

*   **Flexibility:** This approach is more flexible because it allows the user to provide their own `State` implementation, which can include other dependencies besides permissions.
*   **Testability:**  It's easier to test the tools in isolation because you can create mock `State` implementations for testing purposes.
*   **Complexity:** It adds some complexity because you need to define a `State` trait and implement it for your specific use case.
*   **Error Handling:** (Same as before)
*   **Concurrency:** (Same as before)
*   **Integration with `rig` crate:** (Same as before)
*   **`Init` Tool Mutation:** The mechanism for allowing the `Init` tool to mutate the state (e.g., the `to_mutable_state()` method) needs careful design and consideration.

This revised plan provides a more flexible and testable approach to integrating permission management into the `src/tools` implementation by requiring the user to provide a `State` object that exposes the necessary dependencies. Remember to choose a suitable mechanism for allowing the `Init` tool to mutate the state based on your specific needs and constraints.
