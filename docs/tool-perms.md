Okay, here is a step-by-step plan to implement a permission system and shared state management within the `rig`-based `tools` module. This plan creates all necessary components independently within the `tools` module structure.

**Dev Environment**
You can run commands in the developer environment prefixed with `direnv exec <project-dir> <cmd>`.
Examples assuming the working_dir is set to the project root:
- `direnv exec . cargo fmt`
- `direnv exec . cargo test`
- `direnv exec . cargo check`

**Goal:** Refactor the `tools` module (`tools/project.rs`, `tools/file.rs`, `tools/code.rs`) to use a shared `State` object, incorporating a new permission checking system (`SessionPermissions`, `PermissionsRef`) and an `Executor` pattern defined entirely within the `tools` module.

**Context:**
We need to add a security layer to our `rig`-based tools, requiring explicit user grants for file/directory access and command execution. We will manage this using a shared `State` object containing permission information and an execution mechanism. The `rig` library's `Tool` and `ToolEmbedding` traits allow injecting shared state during tool initialization using `Arc<Mutex<>>` for safe concurrent access.

**Core Components to Define (within `tools`):**

1.  `State`: A central struct to hold shared resources like permissions and the executor.
2.  `SessionPermissions` & `PermissionsRef`: Structures and logic for managing read/write/execute permissions.
3.  `Executor` trait & `ShellExecutor`: An abstraction and implementation for executing shell commands safely.
4.  `basic_path_validation`: A function for basic security checks on file paths.

---

## Refactoring Plan

Here are the steps for the junior developer:

### Step 1: Define Core Shared Components within `tools`

**Goal:** Create the central `State` struct and define the interfaces for permissions and execution within the `tools` module.

1.  **Create `tools/shared.rs`:**
    *   Define a new public `State` struct. This will be the container for all shared resources accessible by tools.
    *   Define the public `Executor` trait and the `CommandResult` struct it uses. This outlines how commands will be executed.
    *   Define a type alias `PermissionsRef` using `Arc<Mutex<>>` around a placeholder `SessionPermissions` struct (which will be fully defined in the next step).
    *   Implement a `new` method or `Default` for `State` that initializes its fields (the actual implementations for `PermissionsRef` and `Executor` will be created in subsequent steps).

    ```rust
    // tools/shared.rs
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use std::path::Path;
    use anyhow::Result; // Or Box<dyn std::error::Error> for Executor errors

    // Forward declare or define the permissions struct we'll create later
    // For now, a simple definition or use a placeholder if SessionPermissions is in another file.
    // Let's assume SessionPermissions will be in tools/permissions.rs
    use crate::tools::permissions::SessionPermissions;

    // Type alias for shared, mutable permissions
    pub type PermissionsRef = Arc<Mutex<SessionPermissions>>;

    // Define the result structure for command execution
    #[derive(Debug)]
    pub struct CommandResult {
        pub stdout: String,
        pub stderr: String,
        pub exit_code: i32,
    }

    // Define the trait for executing commands
    #[async_trait::async_trait]
    pub trait Executor {
        async fn execute(
            &self,
            command: String,
            working_dir: Option<&Path>,
        ) -> Result<CommandResult, Box<dyn std::error::Error>; // Ensure error is Send + Sync
    }

    // The central state holder
    #[derive(Clone)]
    pub struct State {
        pub permissions: PermissionsRef,
        pub executor: Arc<dyn Executor + Send + Sync>,
        // Add other shared state if needed later
        // pub project_path: Arc<Mutex<Option<String>>>,
    }

    impl State {
        // Initialize the state. Implementations for permissions and executor
        // will come from subsequent steps.
        pub fn new() -> Self {
            // Create instances of our permission and executor implementations (defined later)
            let permissions = crate::tools::permissions::create_permissions(); // Function to be created in Step 2
            let executor = Arc::new(crate::tools::executor::ShellExecutor::new(permissions.clone())); // Struct to be created in Step 3

            State {
                permissions,
                executor,
            }
        }
    }

    // Add Default trait if useful
    impl Default for State {
        fn default() -> Self {
            Self::new()
        }
    }
    ```
2.  **Update `tools/mod.rs`:**
    *   Make the `shared` module public (`pub mod shared;`).
    *   Also declare `permissions` and `executor` modules (`pub mod permissions; pub mod executor;`).

**Best Practice:** Defining traits (`Executor`) and central state (`State`) first clarifies the architecture. Using `Arc<Mutex<>>` is the standard Rust pattern for shared mutable state in async contexts.

**Compile Check:** Ensure `tools/shared.rs` compiles. There will be errors related to `tools::permissions` and `tools::executor` not existing yet; proceed to the next steps to resolve these.

---

### Step 2: Implement Permission Logic within `tools`

**Goal:** Create the full implementation for `SessionPermissions` and related functions within the `tools` module.

1.  **Create `tools/permissions.rs`:**
    *   Define the public `SessionPermissions` struct. It should contain `HashSet`s for read-allowed directories, write-allowed directories, and allowed command names (strings).
    *   Implement methods on `SessionPermissions`:
        *   `new()`: To initialize with defaults (e.g., an empty set of directories, perhaps some default safe commands like `ls`, `echo`).
        *   `can_read(&self, path: &Path) -> bool`: Checks if the given path falls within an allowed read directory (handle canonicalization).
        *   `can_write(&self, path: &Path) -> bool`: Checks if the given path falls within an allowed write directory (handle canonicalization). Write permission should imply read permission. Check parent directory for file creation/writing.
        *   `can_execute_command(&self, command: &str) -> bool`: Checks if the base command (first word) is in the allowed commands set.
        *   `allow_read(&mut self, dir: PathBuf)`: Adds a directory to the read-allowed set.
        *   `allow_write(&mut self, dir: PathBuf)`: Adds a directory to the write-allowed set (and implicitly read-allowed).
        *   `allow_command(&mut self, command: String)`: Adds a command name to the allowed set.
    *   Implement `basic_path_validation(path: &Path) -> Result<(), String>`. This function should check against a list of sensitive system paths (e.g., `/etc`, `/bin`, `/dev`) and return an error if the input path starts with any of them. Handle path canonicalization.
    *   Implement `create_permissions() -> PermissionsRef`. This function creates a new `SessionPermissions` instance and wraps it in `Arc<Mutex<>>`.

    *Example Structure:*
    ```rust
    // tools/permissions.rs
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tracing::info; // If you want logging similar to mcp

    // Re-import the type alias from shared.rs for clarity
    use crate::tools::shared::PermissionsRef;

    #[derive(Debug, Clone, Default)] // Add Default
    pub struct SessionPermissions {
        read_allowed_dirs: HashSet<PathBuf>,
        write_allowed_dirs: HashSet<PathBuf>,
        allowed_commands: HashSet<String>,
    }

    impl SessionPermissions {
        pub fn new() -> Self {
            // Initialize with default allowed commands if desired
            let mut allowed_commands = HashSet::new();
            allowed_commands.insert("ls".to_string());
            // ... add other safe defaults ...
            Self {
                read_allowed_dirs: HashSet::new(),
                write_allowed_dirs: HashSet::new(),
                allowed_commands,
            }
        }

        pub fn can_read(&self, path: &Path) -> bool {
            // Implement logic: canonicalize path, check against read_allowed_dirs & write_allowed_dirs
            self.has_permission(path, &self.read_allowed_dirs) || self.has_permission(path, &self.write_allowed_dirs)
        }

        pub fn can_write(&self, path: &Path) -> bool {
             // Implement logic: canonicalize path/parent, check against write_allowed_dirs
             // Check the parent directory for file operations
             let check_path = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
             self.has_permission(check_path, &self.write_allowed_dirs)
        }

         // Helper for permission checking (handles canonicalization)
         fn has_permission(&self, path: &Path, allowed_dirs: &HashSet<PathBuf>) -> bool {
             // Implement canonicalization and starts_with check
             // Handle potential canonicalization errors gracefully
             let canonical_path = match path.canonicalize() {
                 Ok(p) => p,
                 Err(_) => {
                    // If path doesn't exist yet (e.g., writing a new file),
                    // check its parent.
                    if let Some(parent) = path.parent() {
                        match parent.canonicalize() {
                             Ok(parent_canon) => {
                                return allowed_dirs.iter().any(|allowed| parent_canon.starts_with(allowed));
                             }
                             Err(_) => return false, // Cannot check parent
                        }
                    } else {
                        return false; // Cannot determine parent
                    }
                 }
             };
             allowed_dirs.iter().any(|allowed| canonical_path.starts_with(allowed))
         }


        pub fn can_execute_command(&self, command: &str) -> bool {
            // Implement logic: split command, check first word against allowed_commands
             let program = command.split_whitespace().next().unwrap_or("");
             self.allowed_commands.contains(program)
        }

        pub fn allow_read(&mut self, dir: PathBuf) {
             info!("Granting read permission for directory: {}", dir.display());
             // Implement logic: canonicalize, insert into read_allowed_dirs
             if let Ok(canonical_path) = dir.canonicalize() {
                 self.read_allowed_dirs.insert(canonical_path);
             } else {
                 self.read_allowed_dirs.insert(dir); // Store as-is if canonicalization fails
             }
        }

        pub fn allow_write(&mut self, dir: PathBuf) {
             info!("Granting write permission for directory: {}", dir.display());
             // Implement logic: canonicalize, insert into write_allowed_dirs AND read_allowed_dirs
             if let Ok(canonical_path) = dir.canonicalize() {
                  self.write_allowed_dirs.insert(canonical_path.clone());
                  self.read_allowed_dirs.insert(canonical_path); // Write implies read
             } else {
                  self.write_allowed_dirs.insert(dir.clone());
                  self.read_allowed_dirs.insert(dir);
             }
        }

        pub fn allow_command(&mut self, command: String) {
            info!("Adding command to allowlist: {}", command);
            // Ensure only the command name (first word) is added if necessary,
            // or allow the full string based on can_execute_command implementation.
            // Assuming can_execute_command checks the first word:
            let program = command.split_whitespace().next().unwrap_or(&command).to_string();
            self.allowed_commands.insert(program);
        }
    }

    // Function to create the shared permissions object
    pub fn create_permissions() -> PermissionsRef {
        Arc::new(Mutex::new(SessionPermissions::new()))
    }

    // Basic path validation function
    pub fn basic_path_validation(path: &Path) -> Result<(), String> {
        // Define dangerous_paths list
        let dangerous_paths = [ /* "/etc", "/bin", ... */ ];
        // Implement logic: canonicalize path_to_check, loop through dangerous_paths, check starts_with
        let path_to_check = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        for dangerous in dangerous_paths.iter() {
             if path_to_check.starts_with(dangerous) {
                 return Err(format!("Access to system directory denied: {}", dangerous));
             }
        }
        Ok(())
    }
    ```

**Best Practice:** Keep permission logic encapsulated. Handle path edge cases (like non-existent paths during a write check) carefully within the `can_write`/`can_read` methods.

**Compile Check:** Ensure `tools/permissions.rs` compiles and that `tools/shared.rs` now compiles successfully as it can find `create_permissions`.

---

### Step 3: Implement Executor Logic within `tools`

**Goal:** Create the `ShellExecutor` implementation for the `Executor` trait defined in `shared.rs`.

1.  **Create `tools/executor.rs`:**
    *   Define the public `ShellExecutor` struct. It should hold a `PermissionsRef` (the one defined in `tools::shared`) to check command execution permissions. It might also cache the detected shell path using an `Arc<Mutex<Option<String>>>`.
    *   Implement `ShellExecutor::new(permissions: PermissionsRef) -> Self`.
    *   Implement the `Executor` trait for `ShellExecutor`. The `execute` method should:
        *   Lock the `PermissionsRef` to check `perms.can_execute_command(&command_str)`. Return an error if not allowed.
        *   If `working_dir` is provided, lock permissions again to check `perms.can_read(working_dir)`. Return an error if not allowed.
        *   Detect the default shell (implement or adapt shell detection logic, e.g., using `whoami`, checking `/etc/passwd` on Unix, `dscl` on macOS, or environment variables like `SHELL`. This logic should be async). Cache the result.
        *   Use `tokio::process::Command` to run the command via the detected shell (e.g., `sh -c "command"` or `cmd /C "command"`).
        *   Set the `current_dir` if `working_dir` is valid.
        *   Capture stdout, stderr, and the exit code.
        *   Return a `CommandResult` on success or a `Box<dyn std::error::Error + Send + Sync>` on failure.

    *Example Structure:*
    ```rust
    // tools/executor.rs
    use crate::tools::shared::{CommandResult, Executor, PermissionsRef};
    use std::path::Path;
    use std::sync::Arc;
    use tokio::process::Command as TokioCommand;
    use tokio::sync::Mutex;
    use anyhow::Result; // Or use Box<dyn Error...> directly

    pub struct ShellExecutor {
        permissions: PermissionsRef,
        shell_path: Arc<Mutex<Option<String>>>, // Cache detected shell
    }

    impl ShellExecutor {
        pub fn new(permissions: PermissionsRef) -> Self {
            Self {
                permissions,
                shell_path: Arc::new(Mutex::new(None)),
            }
        }

        // Helper to detect and cache the shell path
        async fn ensure_shell_initialized(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
             let mut shell_path_guard = self.shell_path.lock().await;
             if shell_path_guard.is_none() {
                 // Implement shell detection logic here (async)
                 // e.g., check env var SHELL, /etc/passwd, etc.
                 // Fallback to "/bin/sh" or "cmd.exe" if detection fails
                 let detected_shell = detect_default_shell().await?; // Implement this function
                 *shell_path_guard = Some(detected_shell);
             }
             // Unwrapping is safe because we initialize if None
             Ok(shell_path_guard.as_ref().unwrap().clone())
        }
    }

    #[async_trait::async_trait]
    impl Executor for ShellExecutor {
        async fn execute(
            &self,
            command_str: String,
            working_dir: Option<&Path>,
        ) -> Result<CommandResult, Box<dyn std::error::Error>> {
            // 1. Check execute permission
            { // Scope for lock
                let perms = self.permissions.lock().await;
                if !perms.can_execute_command(&command_str) {
                    return Err(format!("Command execution denied: '{}'", command_str).into());
                }
            } // Lock released

            // 2. Check working directory read permission if specified
            if let Some(dir) = working_dir {
                 { // Scope for lock
                     let perms = self.permissions.lock().await;
                     if !perms.can_read(dir) {
                          return Err(format!("Read permission denied for working directory: '{}'", dir.display()).into());
                     }
                 } // Lock released
            }


            // 3. Get shell
            let shell = self.ensure_shell_initialized().await?;


            // 4. Prepare command
            let mut command = TokioCommand::new(&shell);
            if cfg!(target_os = "windows") {
                 command.args(["/C", &command_str]);
            } else {
                 command.args(["-c", &command_str]);
            };


            if let Some(dir) = working_dir {
                command.current_dir(dir);
            }

            // 5. Execute and capture output
            let output = command.output().await?; // Propagate IO errors

            // 6. Parse and return result
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1); // Use -1 for signal termination

            Ok(CommandResult { stdout, stderr, exit_code })
        }
    }

    // Helper function for shell detection (must be async)
    async fn detect_default_shell() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Implement platform-specific logic (Unix: check $SHELL, /etc/passwd; Windows: check %COMSPEC%)
        // Fallback safely if detection fails
        #[cfg(unix)]
        {
            if let Ok(shell) = std::env::var("SHELL") {
                if Path::new(&shell).exists() { return Ok(shell); }
            }
            // Add more checks like /etc/passwd if needed
            Ok("/bin/sh".to_string()) // Default fallback for Unix
        }
        #[cfg(windows)]
        {
             if let Ok(shell) = std.env::var("COMSPEC") {
                 if Path::new(&shell).exists() { return Ok(shell); }
             }
             Ok("cmd.exe".to_string()) // Default fallback for Windows
        }
        #[cfg(not(any(unix, windows)))]
        {
             Err("Unsupported OS for shell detection".into())
        }
    }
    ```

**Best Practice:** Encapsulating command execution handles platform differences and permission checks cleanly. Caching the detected shell avoids redundant lookups.

**Compile Check:** Ensure `tools/executor.rs` compiles and that `tools/shared.rs` now fully compiles as it can find `ShellExecutor`.

---

### Step 4: Adapt Tool Structs and Initialization

**Goal:** Modify the `Tool` structs in `tools/*.rs` to hold references to the *newly defined* shared state components (`PermissionsRef`, `Executor`) and initialize them correctly.

1.  **Modify `Tool` Structs:** For *each* tool struct in `tools/project.rs`, `tools/file.rs`, and `tools/code.rs` that needs access to permissions or the executor:
    *   Add fields to hold `PermissionsRef` and/or `Arc<dyn Executor + Send + Sync>`.
    *   Ensure the struct derives `Clone`.
    *   Update imports to use `crate::tools::shared::{State, PermissionsRef, Executor}`.

    *Example for `tools/file.rs::ShowFile`:*
    ```rust
    // tools/file.rs
    use crate::tools::shared::{State, PermissionsRef}; // Use types from tools::shared
    use crate::tools::permissions::basic_path_validation; // Use validation from tools::permissions
    // ... other imports: rig traits, serde, json, PathBuf, fs, Arc, Mutex ...
    use super::project::{FileError, InitError}; // Keep existing error types

    #[derive(Serialize, Deserialize, Clone)] // Add Clone
    pub struct ShowFile {
        permissions: PermissionsRef,
        // No executor needed
    }

    impl Tool for ShowFile { /* ... NAME, Error, Args, Output ... */
        async fn definition(&self, /* ... */) -> ToolDefinition { /* ... */ }
        // Update call() in Step 6
        async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
            // Placeholder - implementation in Step 6
            let path = PathBuf::from(&args.path);
            let content = format!("Placeholder content for {}", path.display());
            Ok(json!({ "content": content }))
        }
    }

    impl ToolEmbedding for ShowFile {
        type InitError = InitError;
        type Context = ();
        type State = State; // Use the State from tools::shared

        fn init(state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
            Ok(ShowFile {
                permissions: state.permissions.clone(), // Clone the Arc from shared State
            })
        }
        // ... embedding_docs, context ...
    }

    // Repeat for other tools, adding `executor: Arc<dyn Executor + Send + Sync>` field
    // where needed (e.g., ExecuteShellCommand).
    ```
2.  **Update `ToolEmbedding::init`:** Implement `init` for each tool to receive `tools::shared::State` and clone the required `Arc`s (`permissions`, `executor`) into the tool instance.
3.  **Update `ToolEmbedding::State`:** Set `type State = State;` using the state defined in `tools::shared`.

**Best Practice:** Use the `init` method provided by `rig`'s `ToolEmbedding` trait for dependency injection.

**Compile Check:** Ensure all tool files (`project.rs`, `file.rs`, `code.rs`) compile after adding the fields and implementing `init` with the new `tools::shared::State`. The `call` methods will still have placeholder logic.

---

### Step 5: Integrate Permissions into `project.rs`

**Goal:** Update `Init` and `RequestPermission` tools to use the `tools::permissions` implementation.

1.  **Modify `tools/project.rs::Init::call`:**
    *   Access `self.permissions`.
    *   Lock the mutex: `let mut perms = self.permissions.lock().await;`.
    *   Call `perms.allow_read(...)` and `perms.allow_write(...)` using the `PathBuf` derived from `args.path`. Handle logic for paths pointing to files vs directories correctly (permissions are typically granted on directories).
    *   Use `crate::tools::permissions::basic_path_validation`.
    *   If `Init` still needs to return a directory tree, implement that logic here (potentially reusing the tree-building code from `tools/file.rs` if refactored, or keeping it duplicated for now). Ensure the tree-building logic *also* respects read permissions by checking `perms.can_read` *before* reading subdirectories (this requires locking `perms` again or passing the locked guard).
    *   Return appropriate `FileError` on failure.

2.  **Modify `tools/project.rs::RequestPermission::call`:**
    *   Access `self.permissions`.
    *   Use `crate::tools::permissions::basic_path_validation`.
    *   Lock the mutex: `let mut perms = self.permissions.lock().await;`.
    *   Based on `args.operation`, call the appropriate `perms.allow_read/allow_write/allow_command` method. Determine the correct `PathBuf` or command `String` to pass based on the operation type.
    *   Return `FileError` on failure (e.g., unknown operation).

    *Example Snippet for `RequestPermission::call`:*
    ```rust
    // Inside RequestPermission::call
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path_buf = PathBuf::from(&args.path);

        // Use validation from tools::permissions
        crate::tools::permissions::basic_path_validation(&path_buf).map_err(FileError)?;

        // Lock permissions
        let mut perms = self.permissions.lock().await;

        let message = match args.operation.as_str() {
            "read" | "write" => {
                // Grant permissions on the directory containing the path
                let dir_path = if path_buf.is_dir() {
                    path_buf.clone()
                } else {
                    path_buf.parent().ok_or_else(|| FileError("Invalid path: no parent directory".to_string()))?.to_path_buf()
                };

                if args.operation == "read" {
                    perms.allow_read(dir_path.clone());
                    format!("Read permission granted for directory: {}", dir_path.display())
                } else { // "write"
                    perms.allow_write(dir_path.clone()); // Assumes write implies read
                    format!("Write permission granted for directory: {}", dir_path.display())
                }
            }
            "execute" => {
                let command = &args.path; // Path is the command here
                // Add the command name (first word) to the allowlist
                let program = command.split_whitespace().next().unwrap_or(command).to_string();
                perms.allow_command(program.clone()); // Use the actual implementation
                format!("Execute permission granted for command: {}", program)
            }
            _ => return Err(FileError(format!("Unknown operation: {}", args.operation))),
        };
        // Unlock happens automatically

        Ok(json!({ "granted": true, "message": message }))
    }
    ```

**Best Practice:** Ensure the correct entity (directory path vs command name) is used when granting permissions. Lock the mutex only for the duration needed.

**Compile Check:** Ensure `tools/project.rs` compiles and correctly interacts with `self.permissions`.

---

### Step 6: Integrate Permissions into `file.rs` Tools

**Goal:** Add permission checks to the file operation tools using `self.permissions`.

1.  **Modify `call` methods:** For `DirectoryTree`, `ShowFile`, `SearchInFile`, `EditFile`, `WriteFile`:
    *   At the beginning of `call`, perform `crate::tools::permissions::basic_path_validation`.
    *   Access `self.permissions` and lock: `let perms = self.permissions.lock().await;`.
    *   Call the appropriate check (`can_read`, `can_write`) based on the tool's function. Remember `WriteFile` likely checks the *parent directory's* write permission.
    *   If the check fails, return a `FileError` stating permission denial and suggesting `request_permission`.
    *   Release the lock (implicitly).
    *   Proceed with the original file operation logic (reading, writing, searching, tree building). Make sure the tree building logic within `DirectoryTree` also checks `can_read` before recursing into subdirectories.

    *Example Snippet for `ShowFile::call`:*
    ```rust
    // Inside ShowFile::call
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path_buf = PathBuf::from(&args.path);
        let start_line = args.start_line.map(|v| v as usize);
        let end_line = args.end_line.map(|v| v as usize);

        // Use validation from tools::permissions
        crate::tools::permissions::basic_path_validation(&path_buf).map_err(FileError)?;

        // --- Permission Check ---
        { // Scope lock guard
            let perms = self.permissions.lock().await;
            if !perms.can_read(&path_buf) { // Use the can_read from tools::permissions
                return Err(FileError(format!(
                    "Read permission denied for path: {}. Use request_permission first.",
                    path_buf.display()
                )));
            }
        } // Lock released
        // --- End Permission Check ---

        // Original file reading logic
        let content = fs::read_to_string(&path_buf)
            .await
            .map_err(|e| FileError(format!("Failed to read file: {}", e)))?;

        // ... Original filtering and JSON response logic ...
        Ok(json!({ /* ... */ }))
    }
    ```
2.  **Update Tool Descriptions:** Modify the `description` in `definition` for each tool to mention the prerequisite `request_permission` call.

**Best Practice:** Perform validation and permission checks *before* IO operations. Clear error messages guide the user.

**Compile Check:** Ensure `tools/file.rs` compiles with validation and permission checks implemented.

---

### Step 7: Refactor `ExecuteShellCommand` using `tools::Executor`

**Goal:** Replace direct command execution with the `tools::executor::ShellExecutor` via `self.executor`.

1.  **Modify `tools/file.rs::ExecuteShellCommand::call`:**
    *   Remove any remaining direct `tokio::process::Command` setup or execution logic.
    *   Remove any manual command allowlist checks (now handled by the executor).
    *   Access the shared executor: `let executor = self.executor.clone();`.
    *   Call `executor.execute(command_str, working_dir.as_deref()).await`.
    *   Map the `Result<CommandResult, Box<dyn Error...>>` from the executor to the tool's `Result<Self::Output, Self::Error>`. Convert the boxed error to `FileError`.
    *   Format the `CommandResult` into the required JSON output.

    *Example Snippet:*
    ```rust
    // Inside ExecuteShellCommand::call
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let command_str = args.command.clone();
        let working_dir = args.working_directory.map(PathBuf::from);

        // Use the executor from self
        match self.executor.execute(command_str, working_dir.as_deref()).await {
            Ok(result) => Ok(json!({
                "stdout": result.stdout,
                "stderr": result.stderr,
                "exit_code": result.exit_code,
                "command": args.command,
                "working_directory": args.working_directory,
                "success": result.exit_code == 0
            })),
            Err(e) => Err(FileError(format!("Command execution failed: {}", e))),
        }
    }
    ```
2.  **Verify `ExecuteShellCommand::init`:** Ensure `init` correctly clones and stores `state.executor` in `self.executor`.

**Best Practice:** Abstracting execution behind the `Executor` trait improves modularity and testability.

**Compile Check:** Ensure `ExecuteShellCommand` compiles and correctly uses `self.executor`.

---

### Step 8: Integrate Permissions into `code.rs` Tools

**Goal:** Add necessary validation and permission checks to code-related tools.

1.  **Modify `tools/code.rs::CodeRepoOverview::call`:**
    *   Perform `crate::tools::permissions::basic_path_validation` on the input path.
    *   Add a read permission check using `self.permissions.lock().await.can_read(&path_buf)`. Return `FileError` if denied.
    *   Ensure `CodeRepoOverview::init` stores `PermissionsRef`.
    *   Keep the existing `yek` integration logic.
    *   Update the tool description in `definition`.

2.  **Modify `tools/code.rs::Search::call`:**
    *   Review the *actual* implementation (not the stub). If it reads from local index files, perform `basic_path_validation` and `can_read` checks on the index directory/files using `self.permissions`.
    *   If it solely interacts with external services or databases without local file access relevant to user permissions, no filesystem permission check might be needed here.
    *   Ensure `Search::init` stores `PermissionsRef` *if* required by the implementation. Update the description.

**Compile Check:** Ensure `tools/code.rs` compiles with required checks.

---

### Step 9: Final Cleanup and Review

**Goal:** Ensure consistency, remove dead code within `tools`, and verify functionality.

1.  **Remove Redundant Code:** Delete any old, unused validation functions or logic within `tools/project.rs`, `tools/file.rs`, etc., ensuring only the versions in `tools/permissions.rs` or `tools/shared.rs` are used.
2.  **Review Error Handling:** Standardize error messages and ensure `FileError` (or appropriate tool-specific errors) are used consistently.
3.  **Review Imports:** Clean up unused imports.
4.  **Review Tool Descriptions:** Verify all tool `description` fields accurately reflect functionality and permission prerequisites.
5.  **Test:** Thoroughly test the permission flows:
    *   Grant permission -> Use tool -> Success.
    *   Use tool without permission -> Denied error.
    *   `Init` grants permissions correctly.
    *   `ExecuteShellCommand` respects command allowlist and working directory read permissions.

---

This standalone plan details how to build the permission and execution system entirely within the `tools` module structure. Remember to commit frequently and test each step.
