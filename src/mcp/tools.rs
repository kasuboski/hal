//! MCP server tools registration and handlers
//!
//! This module registers all available tools with the MCP server and implements
//! their handler functions. It provides:
//!
//! - Permission request tool for granting access to directories and commands
//! - File operation tools: show_file, search_in_file, edit_file, write_file
//! - Shell command execution tool
//! - Standard HAL tools (echo, hello, search)
//!
//! Each tool is defined with an input schema and has a handler function that
//! processes the inputs, performs permission checks, and executes the requested
//! operation.

use mcpr::{error::MCPError, schema::ToolInputSchema, server::Server, transport::Transport, Tool};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::info;

use super::file_utils;
use super::permissions::PermissionsRef;
use super::State;

/// Get all tools available
///
/// Returns a vector of all tools that the HAL MCP server supports.
/// This can be used to initialize the ServerConfig.
///
/// # Returns
///
/// * `Vec<Tool>` - List of all supported tools
pub fn tools() -> Vec<Tool> {
    let mut tools = Vec::new();

    // Permission request tool
    tools.push(Tool {
        name: "request_permission".to_string(),
        description: Some(
            "Request permission before performing operations - use 'read' or 'write' for file access with directory path, or 'execute' with command name as path. Must be called before using other tools.".to_string(),
        ),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(
                [
                    (
                        "operation".to_string(),
                        json!({
                            "type": "string",
                            "enum": ["read", "write", "execute"],
                            "description": "Type of permission to request"
                        }),
                    ),
                    (
                        "path".to_string(),
                        json!({
                            "type": "string",
                            "description": "Path to the directory or file, or in the case of a command: the command to run"
                        }),
                    ),
                ]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
            required: Some(vec!["operation".to_string(), "path".to_string()]),
        },
    });

    // Init tool
    tools.push(Tool {
        name: "init".to_string(),
        description: Some("Initialize the server with a project directory. This will request read and write permissions for the directory. Call this when the user specifies a project or directory to work in. It is helpful to call this before other tools. It will return a directory tree for the project.".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(
                [
                    (
                        "path".to_string(),
                        json!({
                            "type": "string",
                            "description": "Path to the directory to initialize"
                        }),
                    ),
                ]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
            required: Some(vec!["path".to_string()]),
        },
    });

    // Think tool
    tools.push(Tool {
        name: "think".to_string(),
        description: Some("Use the tool to think about something. It will not obtain new information or change the database, but just append the thought to the log. Use it when complex reasoning or some cache memory is needed.".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(
                [
                    (
                        "thought".to_string(),
                        json!({
                            "type": "string",
                            "description": "A thought to think about"
                        }),
                    ),
                ]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
            required: Some(vec!["thought".to_string()]),
        },
    });

    tools.push(Tool {
        name: "directory_tree".to_string(),
        description: Some("Get a directory tree given a path. Returns a list of directories and files in the directory. Requires read permission.".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(
                [
                    (
                        "path".to_string(),
                        json!({
                            "type": "string",
                            "description": "Path to the directory"
                        }),
                    ),
                ]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
            required: Some(vec!["path".to_string()]),
        },
    });

    // Show file tool
    tools.push(Tool {
        name: "show_file".to_string(),
        description: Some("View file contents with optional line range - returns text content. Requires prior read permission via request_permission tool.".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(
                [
                    (
                        "path".to_string(),
                        json!({
                            "type": "string",
                            "description": "Path to the file"
                        }),
                    ),
                    (
                        "start_line".to_string(),
                        json!({
                            "type": "integer",
                            "description": "Starting line number (1-based, optional)"
                        }),
                    ),
                    (
                        "end_line".to_string(),
                        json!({
                            "type": "integer",
                            "description": "Ending line number (inclusive, optional)"
                        }),
                    ),
                ]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
            required: Some(vec!["path".to_string()]),
        },
    });

    // Search in file tool
    tools.push(Tool {
        name: "search_in_file".to_string(),
        description: Some("Search for text patterns or regex in files - returns matching lines with line numbers. Set is_regex=true for regex mode. Requires read permission.".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(
                [
                    (
                        "path".to_string(),
                        json!({
                            "type": "string",
                            "description": "Path to the file"
                        }),
                    ),
                    (
                        "pattern".to_string(),
                        json!({
                            "type": "string",
                            "description": "Search pattern (string or regex)"
                        }),
                    ),
                    (
                        "is_regex".to_string(),
                        json!({
                            "type": "boolean",
                            "description": "Whether to treat pattern as regex (default: false)",
                            "default": false
                        }),
                    ),
                ]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
            required: Some(vec!["path".to_string(), "pattern".to_string()]),
        },
    });

    // Edit file tool
    tools.push(Tool {
        name: "edit_file".to_string(),
        description: Some("Replace text in files - the old_string must match exactly once in the file. Requires write permission. Use search_in_file first to verify uniqueness.".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(
                [
                    (
                        "path".to_string(),
                        json!({
                            "type": "string",
                            "description": "Path to the file"
                        }),
                    ),
                    (
                        "old_string".to_string(),
                        json!({
                            "type": "string",
                            "description": "Text to be replaced (must be unique in the file)"
                        }),
                    ),
                    (
                        "new_string".to_string(),
                        json!({
                            "type": "string",
                            "description": "Text to replace with"
                        }),
                    ),
                ]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
            required: Some(vec![
                "path".to_string(),
                "old_string".to_string(),
                "new_string".to_string(),
            ]),
        },
    });

    // Write file tool
    tools.push(Tool {
        name: "write_file".to_string(),
        description: Some("Create new files or update existing ones - use append=true to add to file instead of overwriting. Creates files if they don't exist. You should read the contents of the file before writing to it. Requires write permission for the directory.".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(
                [
                    (
                        "path".to_string(),
                        json!({
                            "type": "string",
                            "description": "Path to the file"
                        }),
                    ),
                    (
                        "content".to_string(),
                        json!({
                            "type": "string",
                            "description": "Content to write to the file"
                        }),
                    ),
                    (
                        "append".to_string(),
                        json!({
                            "type": "boolean",
                            "description": "Whether to append to the file instead of overwriting (default: false)",
                            "default": false
                        }),
                    ),
                ]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
            required: Some(vec!["path".to_string(), "content".to_string()]),
        },
    });

    // Execute shell command tool
    tools.push(Tool {
        name: "execute_shell_command".to_string(),
        description: Some("Run simple shell commands - returns stdout, stderr, and exit code. Limited to safe commands. Requires execute permission first.".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(
                [
                    (
                        "command".to_string(),
                        json!({
                            "type": "string",
                            "description": "Command to execute"
                        }),
                    ),
                    (
                        "working_directory".to_string(),
                        json!({
                            "type": "string",
                            "description": "Working directory for the command (optional)"
                        }),
                    ),
                ]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
            required: Some(vec!["command".to_string()]),
        },
    });

    // Search tool
    tools.push(Tool {
        name: "search".to_string(),
        description: Some("Search indexed website content using semantic search - returns relevant text chunks with their sources. Used for retrieving information from previously crawled websites.".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(
                [(
                    "query".to_string(),
                    json!({
                        "type": "string",
                        "description": "The search query"
                    }),
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
            required: Some(vec!["query".to_string()]),
        },
    });

    // Code Repo Overview
    tools.push(Tool {
        name: "code_repo_overview".to_string(),
        description: Some("Get an overview of the code repository. Returns a list of files and their contents. This will return a large costly response.".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(
                [
                    (
                        "path".to_string(),
                        json!({
                            "type": "string",
                            "description": "Path to the directory"
                        }),
                    ),
                ]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
            required: Some(vec!["path".to_string()]),
        },
    });

    tools
}

/// Register all tool handlers with the server
///
/// This function registers all available tools with the MCP server:
///
/// 1. Permission management:
///    - `request_permission`: Request access to read/write directories or execute commands
///
/// 2. File operations:
///    - `show_file`: View file contents with optional line range specification
///    - `search_in_file`: Search files using patterns or regex
///    - `edit_file`: Make precise string replacements in files
///    - `write_file`: Create or append content to files
///
/// 3. Shell operations:
///    - `execute_shell_command`: Run commands and return stdout/stderr results
///
/// 4. Standard HAL tools:
///    - `search`: Search previously indexed content
///
/// # Arguments
///
/// * `server` - The MCP server to register handlers with
/// * `permissions` - Shared reference to session permissions
///
/// # Returns
///
/// * `Result<(), MCPError>` - Ok on success, or an MCPError if registration fails
pub fn register_tools<T: Transport + Send + Sync + Clone + 'static>(
    server: &mut Server<T>,
    state: State,
) -> Result<(), MCPError> {
    // Register permission request tool
    register_request_permission_tool(server, state.permissions())?;

    register_project_init_tool(server, state.clone())?;

    register_think_tool(server)?;

    // Register file operation tools
    register_show_file_tool(server, state.permissions())?;
    register_search_in_file_tool(server, state.permissions())?;
    register_edit_file_tool(server, state.permissions())?;
    register_write_file_tool(server, state.permissions())?;
    register_directory_tree_tool(server, state.permissions())?;

    // Register shell command tool
    register_execute_shell_command_tool(server, state.clone())?;

    // Register code repo overview tool
    register_code_repo_overview_tool(server, state.permissions())?;

    // Register the stock HAL tools as well
    register_hal_search_tool(server)?;

    Ok(())
}

fn register_think_tool<T: Transport + Send + Sync + Clone + 'static>(
    server: &mut Server<T>,
) -> Result<(), MCPError> {
    server.register_tool_handler("think", move |_params: Value| async move {
        Ok(json!({
            "output": "thought complete",
        }))
    })
}

fn register_project_init_tool<T: Transport + Send + Sync + Clone + 'static>(
    server: &mut Server<T>,
    state: State,
) -> Result<(), MCPError> {
    server.register_tool_handler("init", move |params: Value| {
        let project_path = state.project_path();
        let perms = state.permissions();
        async move {
            let path = params
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Protocol("Missing path parameter".to_string()))?;

            let path_buf = PathBuf::from(path);

            // Basic validation
            super::permissions::basic_path_validation(&path_buf).map_err(MCPError::Protocol)?;

            // Get parent directory to grant permission to
            let dir_path = if path_buf.is_dir() {
                path_buf.clone()
            } else {
                path_buf
                    .parent()
                    .ok_or_else(|| {
                        MCPError::Protocol("Invalid path: no parent directory".to_string())
                    })?
                    .to_path_buf()
            };

            {
                let perms = &mut *perms.lock().await;
                // Update permissions
                perms.allow_read(dir_path.clone());
                perms.allow_write(dir_path.clone());
            }

            {
                // Update project path
                *project_path.lock().await = Some(path_buf.to_string_lossy().to_string());
            }

            // Use file utility to get directory tree
            let directory_tree = match file_utils::directory_tree(&path_buf, &perms).await {
                Ok(tree) => Ok(json!({
                    "tree": tree,
                    "path": path,
                    "entry_count": tree.len() - 1, // Excluding the root entry
                    "message": format!("Successfully retrieved directory tree for: {}", path)
                })),
                Err(e) => Err(MCPError::Protocol(e)),
            }?;

            Ok(json!({
                "success": true,
                "directory_tree": directory_tree,
                "message": format!("Initialized project: {}", path_buf.display()),
            }))
        }
    })
}

/// Register the request_permission tool
fn register_request_permission_tool<T: Transport + Send + Sync + Clone + 'static>(
    server: &mut Server<T>,
    permissions: PermissionsRef,
) -> Result<(), MCPError> {
    // Register handler
    server.register_tool_handler("request_permission", move |params: Value| {
        let permissions = permissions.clone();
        async move {
            let operation = params
                .get("operation")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Protocol("Missing operation parameter".to_string()))?;

            let path = params
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Protocol("Missing path parameter".to_string()))?;

            let path_buf = PathBuf::from(path);

            // Basic validation
            super::permissions::basic_path_validation(&path_buf)
                .map_err(MCPError::Protocol)?;

            // Get parent directory to grant permission to
            let dir_path = if path_buf.is_dir() {
                path_buf.clone()
            } else {
                path_buf
                    .parent()
                    .ok_or_else(|| MCPError::Protocol("Invalid path: no parent directory".to_string()))?
                    .to_path_buf()
            };

            // Update permissions
            let mut perms = permissions.lock().await;
            match operation {
                "read" => {
                    perms.allow_read(dir_path.clone());
                    Ok(json!({
                        "granted": true,
                        "message": format!("Read permission granted for directory: {}", dir_path.display()),
                    }))
                },
                "write" => {
                    perms.allow_write(dir_path.clone());
                    Ok(json!({
                        "granted": true,
                        "message": format!("Write permission granted for directory: {}", dir_path.display()),
                    }))
                },
                "execute" => {
                    // For execute, we're permitting a command rather than a directory
                    let command = path; // In this case, "path" is actually the command

                    // Extract the program name
                    let program = command.split_whitespace().next()
                        .ok_or_else(|| MCPError::Protocol("Empty command".to_string()))?;

                    perms.allow_command(program.to_string());
                    Ok(json!({
                        "granted": true,
                        "message": format!("Execute permission granted for command: {}", program),
                    }))
                },
                _ => Err(MCPError::Protocol(format!("Unknown operation: {}", operation))),
            }
        }
    })?;

    Ok(())
}

/// Register the show_file tool
fn register_show_file_tool<T: Transport + Send + Sync + Clone + 'static>(
    server: &mut Server<T>,
    permissions: PermissionsRef,
) -> Result<(), MCPError> {
    // Register handler
    server.register_tool_handler("show_file", move |params: Value| {
        let permissions = permissions.clone();
        async move {
            let path = params
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Protocol("Missing path parameter".to_string()))?;

            let start_line = params
                .get("start_line")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            let end_line = params
                .get("end_line")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            let path_buf = PathBuf::from(path);

            // Use file utility to read the file
            match file_utils::show_file(&path_buf, &permissions, start_line, end_line).await {
                Ok(content) => Ok(json!({
                    "content": content,
                    "path": path,
                    "lines": {
                        "start": start_line.unwrap_or(1),
                        "end": end_line,
                        "total": content.lines().count()
                    }
                })),
                Err(e) => Err(MCPError::Protocol(e)),
            }
        }
    })?;

    Ok(())
}

/// Register the search_in_file tool
fn register_search_in_file_tool<T: Transport + Send + Sync + Clone + 'static>(
    server: &mut Server<T>,
    permissions: PermissionsRef,
) -> Result<(), MCPError> {
    // Register handler
    server.register_tool_handler("search_in_file", move |params: Value| {
        let permissions = permissions.clone();
        async move {
            let path = params
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Protocol("Missing path parameter".to_string()))?;

            let pattern = params
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Protocol("Missing pattern parameter".to_string()))?;

            let is_regex = params
                .get("is_regex")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let path_buf = PathBuf::from(path);

            // Use file utility to search the file
            match file_utils::search_in_file(&path_buf, &permissions, pattern, is_regex).await {
                Ok(matches) => Ok(json!({
                    "matches": matches.iter().map(|(line_num, content)| {
                        json!({
                            "line": line_num,
                            "content": content
                        })
                    }).collect::<Vec<_>>(),
                    "pattern": pattern,
                    "is_regex": is_regex,
                    "match_count": matches.len(),
                    "path": path
                })),
                Err(e) => Err(MCPError::Protocol(e)),
            }
        }
    })?;

    Ok(())
}

/// Register the edit_file tool
fn register_edit_file_tool<T: Transport + Send + Sync + Clone + 'static>(
    server: &mut Server<T>,
    permissions: PermissionsRef,
) -> Result<(), MCPError> {
    // Register handler
    server.register_tool_handler("edit_file", move |params: Value| {
        let permissions = permissions.clone();
        async move {
            let path = params
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Protocol("Missing path parameter".to_string()))?;

            let old_string = params
                .get("old_string")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Protocol("Missing old_string parameter".to_string()))?;

            let new_string = params
                .get("new_string")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Protocol("Missing new_string parameter".to_string()))?;

            let path_buf = PathBuf::from(path);

            // Use file utility to edit the file
            match file_utils::edit_file(&path_buf, &permissions, old_string, new_string).await {
                Ok(()) => Ok(json!({
                    "success": true,
                    "path": path,
                    "message": format!("Successfully edited file: {}", path)
                })),
                Err(e) => Err(MCPError::Protocol(e)),
            }
        }
    })?;

    Ok(())
}

/// Register the write_file tool
fn register_write_file_tool<T: Transport + Send + Sync + Clone + 'static>(
    server: &mut Server<T>,
    permissions: PermissionsRef,
) -> Result<(), MCPError> {
    // Register handler
    server.register_tool_handler("write_file", move |params: Value| {
        let permissions = permissions.clone();
        async move {
            let path = params
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Protocol("Missing path parameter".to_string()))?;

            let content = params
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Protocol("Missing content parameter".to_string()))?;

            let append = params
                .get("append")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let path_buf = PathBuf::from(path);

            // Use file utility to write the file
            match file_utils::write_file(&path_buf, &permissions, content, append).await {
                Ok(()) => Ok(json!({
                    "success": true,
                    "path": path,
                    "bytes_written": content.len(),
                    "mode": if append { "append" } else { "overwrite" },
                    "message": format!(
                        "Successfully {} to file: {}",
                        if append { "appended" } else { "wrote" },
                        path
                    )
                })),
                Err(e) => Err(MCPError::Protocol(e)),
            }
        }
    })?;

    Ok(())
}

/// Register the execute_shell_command tool
fn register_execute_shell_command_tool<T: Transport + Send + Sync + Clone + 'static>(
    server: &mut Server<T>,
    state: State,
) -> Result<(), MCPError> {
    // Register handler
    server.register_tool_handler("execute_shell_command", move |params: Value| {
        let executor = state.executor();
        async move {
            let command = params
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Protocol("Missing command parameter".to_string()))?;

            let working_dir = params
                .get("working_directory")
                .and_then(|v| v.as_str())
                .map(PathBuf::from);

            // Execute command
            match executor
                .execute(command.to_string(), working_dir.as_deref())
                .await
            {
                Ok(result) => Ok(json!({
                    "stdout": result.stdout,
                    "stderr": result.stderr,
                    "exit_code": result.exit_code,
                    "command": command,
                    "working_directory": working_dir,
                    "success": result.exit_code == 0
                })),
                Err(e) => Err(MCPError::Protocol(e.to_string())),
            }
        }
    })?;

    Ok(())
}

/// Register the directory_tree tool
fn register_directory_tree_tool<T: Transport + Send + Sync + Clone + 'static>(
    server: &mut Server<T>,
    permissions: PermissionsRef,
) -> Result<(), MCPError> {
    // Register handler
    server.register_tool_handler("directory_tree", move |params: Value| {
        let permissions = permissions.clone();
        async move {
            let path = params
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Protocol("Missing path parameter".to_string()))?;

            let path_buf = PathBuf::from(path);

            // Use file utility to get directory tree
            match file_utils::directory_tree(&path_buf, &permissions).await {
                Ok(tree) => Ok(json!({
                    "tree": tree,
                    "path": path,
                    "entry_count": tree.len() - 1, // Excluding the root entry
                    "message": format!("Successfully retrieved directory tree for: {}", path)
                })),
                Err(e) => Err(MCPError::Protocol(e)),
            }
        }
    })?;

    Ok(())
}

/// Register the code_repo_overview tool
fn register_code_repo_overview_tool<T: Transport + Send + Sync + Clone + 'static>(
    server: &mut Server<T>,
    permissions: PermissionsRef,
) -> Result<(), MCPError> {
    // Register handler
    server.register_tool_handler("code_repo_overview", move |params: Value| {
        let permissions = permissions.clone();
        async move {
            let path = params
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Protocol("Missing path parameter".to_string()))?;

            let path_buf = PathBuf::from(path);

            // Check read permission
            let perms = permissions.lock().await;
            if !perms.can_read(&path_buf) {
                return Err(MCPError::Protocol(format!(
                    "No read permission for directory: {}",
                    path_buf.display()
                )));
            }
            // Release the lock
            drop(perms);

            // Create YekConfig with tokens mode
            let mut config = yek::config::YekConfig::default();
            let ignore = yek::defaults::DEFAULT_IGNORE_PATTERNS
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            config.ignore_patterns = ignore;
            config.input_paths = vec![path.to_string()];
            config.token_mode = true;
            config.max_size = "10K".to_string();
            config.tokens = "10000".to_string();

            config
                .validate()
                .map_err(|e| MCPError::Protocol(format!("failed to validate config: {e}")))?;

            // Use spawn_blocking since overview is synchronous
            let config_clone = config.clone();
            let result = tokio::task::spawn_blocking(move || super::code::overview(&config_clone))
                .await
                .map_err(|e| MCPError::Protocol(format!("Failed to run overview: {}", e)))?;

            match result {
                Ok((overview, files)) => Ok(json!({
                    "overview": overview,
                    "files": files.len(),
                    "path": path
                })),
                Err(e) => Err(MCPError::Protocol(format!(
                    "Failed to generate overview: {}",
                    e
                ))),
            }
        }
    })?;

    Ok(())
}

fn register_hal_search_tool<T: Transport + Send + Sync + Clone + 'static>(
    server: &mut Server<T>,
) -> Result<(), MCPError> {
    // Register search handler that uses the HAL search functionality
    server.register_tool_handler("search", |params: Value| async move {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Protocol("Missing query parameter".to_string()))?;

        info!("Search request: {}", query);

        // Create database connection
        let db = match crate::index::Database::new_local_libsql().await {
            Ok(db) => db,
            Err(e) => {
                return Err(MCPError::Protocol(format!(
                    "Failed to connect to database: {}",
                    e
                )))
            }
        };

        let client = crate::model::Client::new_gemini_free_from_env();

        // Create search options
        let options = crate::search::SearchOptions {
            limit: 5,
            source_filter: None,
            date_range: None,
        };

        // Search the index
        let results =
            match crate::search::search_index_with_client(&db, &client, query, options).await {
                Ok(results) => results,
                Err(e) => return Err(MCPError::Protocol(format!("Search failed: {}", e))),
            };

        // Format results
        let formatted_results = results
            .iter()
            .map(|r| {
                json!({
                    "text": r.text,
                    "url": r.url,
                    "context": r.context
                })
            })
            .collect::<Vec<_>>();

        Ok(json!({
            "results": formatted_results
        }))
    })?;

    Ok(())
}
