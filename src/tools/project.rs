use anyhow::Result;
use rig::{
    completion::ToolDefinition,
    tool::{Tool, ToolEmbedding},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use tracing::info;

// Common error types - could be moved to a common module
#[derive(Debug, thiserror::Error)]
#[error("File operation error: {0}")]
pub struct FileError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("Init error")]
pub struct InitError;

// Parameter structs for project tools
#[derive(Deserialize)]
pub struct PathParam {
    pub path: String,
}

#[derive(Deserialize)]
pub struct PermissionParams {
    pub operation: String,
    pub path: String,
}

#[derive(Deserialize)]
pub struct ThoughtParams {
    pub thought: String,
}

// Basic validation to prevent access to system directories
pub fn basic_path_validation(path: &Path) -> Result<(), String> {
    // Check for obviously dangerous paths
    let dangerous_paths = [
        "/etc",
        "/bin",
        "/sbin",
        "/usr/bin",
        "/usr/sbin",
        "/boot",
        "/lib",
        "/lib64",
        "/dev",
        "/proc",
        "/sys",
        "/var/run",
        "/var/log",
        "/var/lib",
        "/var/tmp",
    ];

    // Convert to absolute path if possible
    let path_to_check = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    for dangerous in dangerous_paths.iter() {
        if path_to_check.starts_with(dangerous) {
            return Err(format!("Cannot access system directory: {}", dangerous));
        }
    }

    Ok(())
}

// Init Tool
#[derive(Serialize, Deserialize)]
pub struct Init;

impl Tool for Init {
    const NAME: &'static str = "init";

    type Error = FileError;
    type Args = PathParam;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "init",
            "description": "Initialize the server with a project directory. This will request read and write permissions for the directory. Call this when the user specifies a project or directory to work in. It is helpful to call this before other tools. It will return a directory tree for the project.",
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

        // Basic validation
        if let Err(e) = basic_path_validation(&path_buf) {
            return Err(FileError(e));
        }

        // Get parent directory to grant permission to
        let dir_path = if path_buf.is_dir() {
            path_buf.clone()
        } else {
            path_buf
                .parent()
                .ok_or_else(|| FileError("Invalid path: no parent directory".to_string()))?
                .to_path_buf()
        };

        // Log the permissions being granted
        info!(
            "Granting read permission for directory: {}",
            dir_path.display()
        );
        info!(
            "Granting write permission for directory: {}",
            dir_path.display()
        );

        // Store the project path for future reference (though we don't have the same state management)
        info!("Project initialized: {}", path_buf.display());

        // Use file utility to get directory tree
        let result = match crate::tools::file::build_tree_structure(
            &path_buf,
            &mut vec![],
            String::from(""),
            0,
        )
        .await
        {
            Ok(_) => {
                // Create a new tree with the right format for display
                let mut tree = Vec::new();
                let root_name = path_buf
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path_buf.to_string_lossy().to_string());

                tree.push(root_name);
                if let Err(e) = crate::tools::file::build_tree_structure(
                    &path_buf,
                    &mut tree,
                    String::from("  "),
                    1,
                )
                .await
                {
                    return Err(FileError(e));
                }

                Ok(json!({
                    "tree": tree,
                    "path": args.path,
                    "entry_count": tree.len() - 1,
                    "message": format!("Successfully retrieved directory tree for: {}", args.path)
                }))
            }
            Err(e) => Err(FileError(e)),
        }?;

        Ok(json!({
            "success": true,
            "directory_tree": result,
            "message": format!("Initialized project: {}", path_buf.display()),
        }))
    }
}

impl ToolEmbedding for Init {
    type InitError = InitError;
    type Context = ();
    type State = ();

    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(Init)
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec![
            "Initialize a project directory".into(),
            "Set up permissions for a project".into(),
            "Get a directory tree for a project".into(),
        ]
    }

    fn context(&self) -> Self::Context {}
}

// RequestPermission Tool
#[derive(Serialize, Deserialize)]
pub struct RequestPermission;

impl Tool for RequestPermission {
    const NAME: &'static str = "request_permission";

    type Error = FileError;
    type Args = PermissionParams;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "request_permission",
            "description": "Request permission before performing operations - use 'read' or 'write' for file access with directory path, or 'execute' with command name as path. Must be called before using other tools.",
            "parameters": {
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["read", "write", "execute"],
                        "description": "Type of permission to request"
                    },
                    "path": {
                        "type": "string",
                        "description": "Path to the directory or file, or in the case of a command: the command to run"
                    }
                },
                "required": ["operation", "path"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path_buf = PathBuf::from(&args.path);

        // Use our basic path validation
        if let Err(e) = basic_path_validation(&path_buf) {
            return Err(FileError(e));
        }

        // Get parent directory to grant permission to
        let dir_path = if path_buf.is_dir() {
            path_buf.clone()
        } else {
            path_buf
                .parent()
                .ok_or_else(|| FileError("Invalid path: no parent directory".to_string()))?
                .to_path_buf()
        };

        // Log the permission request (similar to what happens in the MCP implementation)
        let message = match args.operation.as_str() {
            "read" => {
                info!(
                    "Granting read permission for directory: {}",
                    dir_path.display()
                );
                format!(
                    "Read permission granted for directory: {}",
                    dir_path.display()
                )
            }
            "write" => {
                info!(
                    "Granting write permission for directory: {}",
                    dir_path.display()
                );
                format!(
                    "Write permission granted for directory: {}",
                    dir_path.display()
                )
            }
            "execute" => {
                // For execute, we're permitting a command rather than a directory
                let command = &args.path; // In this case, "path" is actually the command

                // Extract the program name
                let program = command
                    .split_whitespace()
                    .next()
                    .ok_or_else(|| FileError("Empty command".to_string()))?;

                info!("Adding command to allowlist: {}", program);
                format!("Execute permission granted for command: {}", program)
            }
            _ => return Err(FileError(format!("Unknown operation: {}", args.operation))),
        };

        Ok(json!({
            "granted": true,
            "message": message
        }))
    }
}

impl ToolEmbedding for RequestPermission {
    type InitError = InitError;
    type Context = ();
    type State = ();

    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(RequestPermission)
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

// Think Tool
#[derive(Serialize, Deserialize)]
pub struct Think;

impl Tool for Think {
    const NAME: &'static str = "think";

    type Error = FileError;
    type Args = ThoughtParams;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "think",
            "description": "Use the tool to think about something. It will not obtain new information or change the database, but just append the thought to the log. Use it when complex reasoning or some cache memory is needed.",
            "parameters": {
                "type": "object",
                "properties": {
                    "thought": {
                        "type": "string",
                        "description": "A thought to think about"
                    }
                },
                "required": ["thought"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Log the thought
        info!("Thought: {}", args.thought);

        // Just return success, matches the MCP implementation
        Ok(json!({
            "output": "thought complete"
        }))
    }
}

impl ToolEmbedding for Think {
    type InitError = InitError;
    type Context = ();
    type State = ();

    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(Think)
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec![
            "Think through complex problems".into(),
            "Reason step by step".into(),
            "Cache information for later use".into(),
        ]
    }

    fn context(&self) -> Self::Context {}
}
