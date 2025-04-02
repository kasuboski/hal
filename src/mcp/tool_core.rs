//! Core tools implementation using RMCP attribute macros
//!
//! This module contains core tools like think, permission request, and init
//! using the new RMCP attribute macro pattern.

use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

use rmcp::{
    Error,
    handler::server::tool::ToolBox,
    model::{CallToolResult, Content},
    schemars, tool,
};

use crate::mcp::file_utils;
use crate::mcp::permissions::PermissionsRef;
use tokio::sync::Mutex;

/// Core tools handler implementing basic utility tools
#[derive(Clone)]
#[allow(dead_code)] // Fields are used indirectly by RMCP macros
pub struct CoreTools {
    /// Permissions manager
    permissions: PermissionsRef,
    /// Project path for context
    project_path: Arc<Mutex<Option<String>>>,
}

#[tool(tool_box)]
impl CoreTools {
    /// Create a new CoreTools instance with the necessary dependencies
    pub fn new(permissions: PermissionsRef, project_path: Arc<Mutex<Option<String>>>) -> Self {
        Self {
            permissions,
            project_path,
        }
    }

    pub fn get_tool_box() -> &'static ToolBox<Self> {
        // Calls the associated function generated by #[tool(tool_box)]
        Self::tool_box()
    }

    /// The think tool allows the AI to reason through complex problems
    #[tool(
        description = "Use the tool to think about something. It will not obtain new information or change the database, but just append the thought to the log. Use it when complex reasoning or some cache memory is needed. Useful for multi-step planning or reasoning through complicated problems."
    )]
    fn think(
        &self,
        #[tool(param)]
        #[schemars(description = "A thought to think about")]
        thought: String,
    ) -> Result<CallToolResult, Error> {
        // Parameter validation
        if thought.trim().is_empty() {
            return Err(Error::invalid_request("Thought cannot be empty", None));
        }

        // Log the tool call
        tracing::info!("Thinking about something");
        tracing::info!(thought = %thought, "Thought process");

        // Return a more structured response
        let response = json!({
            "success": true,
            "thought_logged": true,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "message": "Thought recorded successfully"
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&response).unwrap(),
        )]))
    }

    /// The finish tool allows the AI signal it is done with the current task
    #[tool(
        description = "Finish the task by summarizing the results. This tool will end the current conversation. Use this when you have completed the current task and want to signal completion to the user. Include a summary of what was accomplished."
    )]
    fn finish(
        &self,
        #[tool(param)]
        #[schemars(description = "The summary of the task process and results.")]
        summary: String,
    ) -> Result<CallToolResult, Error> {
        // Parameter validation
        if summary.trim().is_empty() {
            return Err(Error::invalid_request(
                "Summary cannot be empty. Please provide a brief description of the task results.",
                None,
            ));
        }

        // Log the tool call
        tracing::info!(summary = %summary, "Task finished");

        // Return a more structured response
        let response = json!({
            "success": true,
            "task_completed": true,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "summary": summary,
            "message": "Task completed successfully"
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&response).unwrap(),
        )]))
    }

    /// Request permission for various operations
    #[tool(
        description = "Request permission before performing operations - use 'read' or 'write' for file access with directory path, or 'execute' with command name as path. Must be called before using other tools. For file operations, permissions apply to the specified directory and all its contents. For commands, permission applies to the specific command only."
    )]
    async fn request_permission(
        &self,
        #[tool(param)]
        #[schemars(description = "Type of permission to request")]
        operation: String,

        #[tool(param)]
        #[schemars(
            description = "Path to the directory or file, or in the case of a command: the command to run"
        )]
        path: String,
    ) -> Result<CallToolResult, Error> {
        let path_buf = PathBuf::from(&path);

        // Basic validation
        super::permissions::basic_path_validation(&path_buf)
            .map_err(|e| Error::invalid_request(e, None))?;

        // Get parent directory to grant permission to
        let dir_path = if path_buf.is_dir() {
            path_buf.clone()
        } else {
            path_buf
                .parent()
                .ok_or_else(|| Error::invalid_request("Invalid path: no parent directory", None))?
                .to_path_buf()
        };

        // Validate operation type explicitly
        if !["read", "write", "execute"].contains(&operation.as_str()) {
            return Err(Error::invalid_request(
                format!(
                    "Unknown operation: '{}'. Must be 'read', 'write', or 'execute'",
                    operation
                ),
                None,
            ));
        }

        // Update permissions
        let mut perms = self.permissions.lock().await;
        let (result, details) = match operation.as_str() {
            "read" => {
                // Check if the directory exists before granting permission
                let exists = dir_path.exists();
                let status = if exists { "existing" } else { "specified" };

                perms.allow_read(dir_path.clone());
                (
                    format!(
                        "Read permission granted for {} directory: {}",
                        status,
                        dir_path.display()
                    ),
                    json!({
                        "operation": "read",
                        "path": dir_path.to_string_lossy(),
                        "exists": exists,
                        "absolute_path": dir_path.canonicalize()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|_| dir_path.to_string_lossy().to_string())
                    }),
                )
            }
            "write" => {
                // Check if the directory exists before granting permission
                let exists = dir_path.exists();
                let status = if exists { "existing" } else { "specified" };

                perms.allow_write(dir_path.clone());
                (
                    format!(
                        "Write permission granted for {} directory: {}",
                        status,
                        dir_path.display()
                    ),
                    json!({
                        "operation": "write",
                        "path": dir_path.to_string_lossy(),
                        "exists": exists,
                        "absolute_path": dir_path.canonicalize()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|_| dir_path.to_string_lossy().to_string()),
                        "includes_read": true
                    }),
                )
            }
            "execute" => {
                let program = path
                    .split_whitespace()
                    .next()
                    .ok_or_else(|| Error::invalid_request("Empty command", None))?;
                perms.allow_command(program.to_string());
                (
                    format!("Execute permission granted for command: {}", program),
                    json!({
                        "operation": "execute",
                        "command": program,
                    }),
                )
            }
            _ => unreachable!(), // We already validated above
        };

        let response = json!({
            "success": true,
            "message": result,
            "details": details
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&response).unwrap(),
        )]))
    }

    /// Initialize the server with a project directory
    #[tool(
        description = "Initialize the server with a project directory. This will request read and write permissions for the directory. Call this when the user specifies a project or directory to work in. It is helpful to call this before other tools. It will return a directory tree for the project. This is typically the first tool you should call when working with a codebase."
    )]
    async fn init(
        &self,
        #[tool(param)]
        #[schemars(description = "Path to the directory to initialize")]
        path: String,
    ) -> Result<CallToolResult, Error> {
        let path_buf = PathBuf::from(&path);

        // Basic validation
        super::permissions::basic_path_validation(&path_buf)
            .map_err(|e| Error::invalid_request(e, None))?;

        // Validate if path exists
        if !path_buf.exists() {
            return Err(Error::invalid_request(
                format!(
                    "Path does not exist: {}. Please specify an existing directory.",
                    path_buf.display()
                ),
                None,
            ));
        }

        // Validate if path is a directory
        if !path_buf.is_dir() {
            return Err(Error::invalid_request(
                format!(
                    "Path is not a directory: {}. Please specify a directory, not a file.",
                    path_buf.display()
                ),
                None,
            ));
        }

        let dir_path = if path_buf.is_dir() {
            path_buf.clone()
        } else {
            path_buf
                .parent()
                .ok_or_else(|| Error::invalid_request("Invalid path: no parent directory", None))?
                .to_path_buf()
        };

        // Get canonicalized path if possible for better error messages
        let canonical_path = dir_path.canonicalize().unwrap_or_else(|_| dir_path.clone());

        // Grant permissions
        {
            let perms = &mut *self.permissions.lock().await;
            perms.allow_read(dir_path.clone());
            perms.allow_write(dir_path.clone());
        }

        // Store project path
        {
            *self.project_path.lock().await = Some(path_buf.to_string_lossy().to_string());
        }

        // Get directory tree
        match file_utils::directory_tree(&path_buf, &self.permissions).await {
            Ok(tree) => {
                // Count directories and files separately for better statistics
                let dirs_count = tree
                    .iter()
                    .filter(|line| line.contains("/") || line.contains("\\"))
                    .count();

                let files_count = (tree.len() - 1) - dirs_count;

                let directory_tree = json!({
                    "tree": tree,
                    "path": path,
                    "stats": {
                        "total_entries": tree.len() - 1,  // Exclude root entry
                        "directories": dirs_count,
                        "files": files_count,
                        "skipped_entries": tree.iter()
                            .filter(|line| line.contains("[Skipped]"))
                            .count()
                    },
                    "message": format!("Successfully retrieved directory tree for: {}", path)
                });

                // Create a combined response with init success and tree data
                let response = json!({
                    "success": true,
                    "project_initialized": true,
                    "project_path": canonical_path.to_string_lossy(),
                    "permissions_granted": {
                        "read": true,
                        "write": true
                    },
                    "directory_tree": directory_tree
                });

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string(&response).unwrap(),
                )]))
            }
            Err(e) => {
                // Even if tree generation fails, initialization succeeded
                let response = json!({
                    "success": true,
                    "project_initialized": true,
                    "project_path": canonical_path.to_string_lossy(),
                    "permissions_granted": {
                        "read": true,
                        "write": true
                    },
                    "directory_tree_error": format!("Failed to generate directory tree: {}", e)
                });

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string(&response).unwrap(),
                )]))
            }
        }
    }
}
