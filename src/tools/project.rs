use anyhow::Result;
use rig::{
    completion::ToolDefinition,
    tool::{Tool, ToolEmbedding},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use tracing::info;

// Import from new locations within the tools module
use super::error::{FileError, InitError};
use super::permissions::basic_path_validation;
use super::shared::{PermissionsRef, State};

// Parameter structs remain the same
#[derive(Deserialize)]
pub struct PathParam {
    pub path: String,
}

#[derive(Deserialize)]
pub struct PermissionParams {
    pub operation: String, // "read", "write", "execute"
    pub path: String,      // Directory path for read/write, command string for execute
}

#[derive(Deserialize)]
pub struct ThoughtParams {
    pub thought: String,
}

// Remove the old basic_path_validation from this file
// pub fn basic_path_validation(path: &Path) -> Result<(), String> { ... } // DELETE THIS

// Init Tool
#[derive(Serialize, Deserialize, Clone)] // Added Clone
pub struct Init {
    // Add permissions field
    #[serde(skip)]
    permissions: PermissionsRef,
    // We might need project_path from state if we want to store it,
    // but the original rig version didn't seem to use it beyond logging.
    // Let's omit it for now unless explicitly needed later.
    // #[serde(skip)]
    // project_path_state: Arc<tokio::sync::Mutex<Option<String>>>,
}

impl Tool for Init {
    const NAME: &'static str = "init";

    type Error = FileError;
    type Args = PathParam;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "init",
            // Updated description
            "description": "Initialize the server with a project directory. This implicitly grants read and write permissions for the specified directory. Call this when the user specifies a project or directory to work in. Returns a directory tree for the project. Requires read permission to generate the tree.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the directory to initialize"
                    }
                },
                "required": ["path"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path_buf = PathBuf::from(&args.path);

        // --- Validation ---
        basic_path_validation(&path_buf).map_err(FileError)?;

        // Ensure path exists and is a directory before granting permissions/building tree
        if !path_buf.exists() {
            return Err(FileError(format!(
                "Path does not exist: {}",
                path_buf.display()
            )));
        }
        if !path_buf.is_dir() {
            // Allow initializing with a file path? Let's assume directory for now.
            // If file, maybe init with parent dir? Clarify requirements if needed.
            return Err(FileError(format!(
                "Path is not a directory: {}",
                path_buf.display()
            )));
        }

        // Grant permissions for the specified directory
        {
            let mut perms = self.permissions.lock().await;
            // Grant read and write for the exact directory provided
            info!(
                "Granting read permission for directory: {}",
                path_buf.display()
            );
            perms.allow_read(path_buf.clone());
            info!(
                "Granting write permission for directory: {}",
                path_buf.display()
            );
            perms.allow_write(path_buf.clone()); // Write implies read
        } // Lock released

        // Store the project path? Not currently used by other tools via State.
        info!("Project initialized with path: {}", path_buf.display());

        // Generate the directory tree (requires read permission, which we just granted)
        let mut tree_result = Vec::new();
        let root_name = path_buf
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path_buf.to_string_lossy().to_string());

        tree_result.push(root_name);

        // Call the tree builder from file.rs, passing permissions
        match super::file::build_tree_structure(
            &path_buf,
            &self.permissions, // Pass the permissions Arc
            &mut tree_result,
            String::from("  "),
            1, // Start depth at 1 for children
        )
        .await
        {
            Ok(_) => {
                // Tree successfully built (or partially built if sub-perms were denied)
                let tree_json = json!({
                    "tree": tree_result,
                    "path": args.path,
                    "entry_count": tree_result.len() - 1,
                    "message": format!("Successfully retrieved directory tree for: {}", args.path)
                });

                Ok(json!({
                    "success": true,
                    "directory_tree": tree_json, // Embed the tree result
                    "message": format!("Initialized project and granted permissions for: {}", path_buf.display()),
                }))
            }
            Err(e) => {
                // Error during tree building (e.g., IO error despite permission)
                Err(FileError(format!("Failed to build directory tree: {}", e)))
            }
        }
    }
}

impl ToolEmbedding for Init {
    type InitError = InitError;
    type Context = ();
    type State = State; // Use shared State

    fn init(state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(Init {
            permissions: state.permissions.clone(), // Clone Arc
                                                    // project_path_state: state.project_path(), // If needed later
        })
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec![
            "Initialize a project directory".into(),
            "Set up permissions for a project directory".into(),
            "Get a directory tree for a project".into(),
        ]
    }

    fn context(&self) -> Self::Context {}
}

// RequestPermission Tool
#[derive(Serialize, Deserialize, Clone)] // Added Clone
pub struct RequestPermission {
    // Add permissions field
    #[serde(skip)]
    permissions: PermissionsRef,
}

impl Tool for RequestPermission {
    const NAME: &'static str = "request_permission";

    type Error = FileError;
    type Args = PermissionParams;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "request_permission",
            // Description remains largely the same, maybe slightly clearer
            "description": "Request permission before performing operations. Use 'read' or 'write' with a directory path to grant access to that directory and its contents. Use 'execute' with a command name (e.g., 'cargo') to allow running that command.",
            "parameters": {
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["read", "write", "execute"],
                        "description": "Type of permission: 'read', 'write' (for directories), or 'execute' (for commands)"
                    },
                    "path": {
                        "type": "string",
                        "description": "For 'read'/'write': the directory path. For 'execute': the command name (e.g., 'ls', 'cargo', 'python')."
                    }
                },
                "required": ["operation", "path"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Lock permissions
        let mut perms = self.permissions.lock().await;

        let message = match args.operation.as_str() {
            "read" | "write" => {
                let path_buf = PathBuf::from(&args.path);

                // --- Validation (only for read/write paths) ---
                basic_path_validation(&path_buf).map_err(FileError)?;

                // Determine the directory to grant permission for.
                // If a file path is given, grant permission for its parent directory.
                // If a dir path is given, grant permission for that directory.
                let dir_to_allow = if path_buf.is_file() {
                    path_buf.parent().ok_or_else(|| {
                        FileError(format!(
                            "Cannot determine parent directory for path: {}",
                            path_buf.display()
                        ))
                    })?
                } else {
                    // Assume it's a directory path or non-existent (allow granting for potential future dir)
                    &path_buf
                };

                // Canonicalize *before* inserting if possible, otherwise store as is.
                let canonical_dir = dir_to_allow
                    .canonicalize()
                    .unwrap_or_else(|_| dir_to_allow.to_path_buf());

                if args.operation == "read" {
                    info!(
                        "Granting read permission for directory: {}",
                        canonical_dir.display()
                    );
                    perms.allow_read(canonical_dir.clone());
                    format!(
                        "Read permission granted for directory: {}",
                        canonical_dir.display()
                    )
                } else {
                    // "write"
                    info!(
                        "Granting write permission for directory: {}",
                        canonical_dir.display()
                    );
                    perms.allow_write(canonical_dir.clone()); // Write implies read
                    format!(
                        "Write permission granted for directory: {}",
                        canonical_dir.display()
                    )
                }
            }
            "execute" => {
                // Path validation doesn't apply to command names
                let command = &args.path;

                // Extract the program name (first word) to allowlist
                let program = command.split_whitespace().next().unwrap_or(command).trim();

                if program.is_empty() {
                    return Err(FileError(
                        "Cannot grant execute permission for an empty command string".to_string(),
                    ));
                }

                info!("Adding command to allowlist: {}", program);
                perms.allow_command(program.to_string());
                format!("Execute permission granted for command: {}", program)
            }
            _ => {
                return Err(FileError(format!(
                    "Unknown operation type: '{}'. Must be 'read', 'write', or 'execute'.",
                    args.operation
                )))
            }
        };
        // Unlock happens automatically when perms goes out of scope

        Ok(json!({
            "granted": true,
            "message": message
        }))
    }
}

impl ToolEmbedding for RequestPermission {
    type InitError = InitError;
    type Context = ();
    type State = State; // Use shared State

    fn init(state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(RequestPermission {
            permissions: state.permissions.clone(), // Clone Arc
        })
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec![
            "Request permission before performing operations".into(),
            "Grant read access to a directory".into(),
            "Grant write access to a directory".into(),
            "Grant execution permission for a command".into(),
        ]
    }

    fn context(&self) -> Self::Context {}
}

// Think Tool - No changes needed for permissions
#[derive(Serialize, Deserialize, Clone)] // Added Clone
pub struct Think;

impl Tool for Think {
    const NAME: &'static str = "think";

    type Error = FileError; // Keep FileError for consistency, though it won't be used
    type Args = ThoughtParams;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "think",
            "description": "Log a thought process or plan. Does not interact with files, permissions, or external state. Useful for outlining steps or reasoning.",
            "parameters": {
                "type": "object",
                "properties": {
                    "thought": {
                        "type": "string",
                        "description": "The thought or reasoning step to log."
                    }
                },
                "required": ["thought"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Log the thought using tracing
        info!(thought = %args.thought, "Think tool called");

        // Return success, no external effects
        Ok(json!({
            "output": "Thought logged successfully." // Slightly more descriptive output
        }))
    }
}

impl ToolEmbedding for Think {
    type InitError = InitError;
    type Context = ();
    // Think doesn't need state, but the trait requires it.
    // We can use the shared State type but just ignore it in init.
    type State = State;

    // Init takes state but doesn't store it
    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(Think)
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec![
            "Think through complex problems".into(),
            "Reason step by step".into(),
            "Log intermediate reasoning".into(), // Updated doc string
        ]
    }

    fn context(&self) -> Self::Context {}
}

#[derive(Serialize, Deserialize, Clone)] // Added Clone
pub struct Finish;

#[derive(Serialize, Deserialize, Clone)]
pub struct FinishParams {
    pub summary: String,
}

impl Tool for Finish {
    const NAME: &'static str = "finish";

    type Error = FileError; // Keep FileError for consistency, though it won't be used
    type Args = FinishParams;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "finish",
            "description": "Finish the task by summarizing the results. This tool will end the current conversation.",
            "parameters": {
                "type": "object",
                "properties": {
                    "summary": {
                        "type": "string",
                        "description": "The summary of the task process and results."
                    }
                },
                "required": ["summary"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Log the summary using tracing
        info!(summary = %args.summary, "finish tool called");

        // Return success, no external effects
        Ok(json!({
            "output": "summary logged successfully."
        }))
    }
}

impl ToolEmbedding for Finish {
    type InitError = InitError;
    type Context = ();
    // We can use the shared State type but just ignore it in init.
    type State = State;

    // Init takes state but doesn't store it
    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(Finish)
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec![
            "End the conversation".into(),
            "Summarize the results".into(),
            "Summarize the task process".into(), // Updated doc string
        ]
    }

    fn context(&self) -> Self::Context {}
}
