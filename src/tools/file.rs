use anyhow::Result;
use regex::Regex;
use rig::{
    completion::ToolDefinition,
    tool::{Tool, ToolEmbedding},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

// Importing common error types from project module
use super::project::{FileError, InitError};

// Parameter structs for file tools
#[derive(Deserialize)]
pub struct OptionalLineRange {
    pub path: String,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub path: String,
    pub pattern: String,
    pub is_regex: Option<bool>,
}

#[derive(Deserialize)]
pub struct EditParams {
    pub path: String,
    pub old_string: String,
    pub new_string: String,
}

#[derive(Deserialize)]
pub struct WriteParams {
    pub path: String,
    pub content: String,
    pub append: Option<bool>,
}

#[derive(Deserialize)]
pub struct CommandParams {
    pub command: String,
    pub working_directory: Option<String>,
}

// DirectoryTree Tool
#[derive(Serialize, Deserialize)]
pub struct DirectoryTree;

impl Tool for DirectoryTree {
    const NAME: &'static str = "directory_tree";

    type Error = FileError;
    type Args = super::project::PathParam;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "directory_tree",
            "description": "Get a directory tree given a path. Returns a list of directories and files in the directory. Requires read permission.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the directory"
                    }
                },
                "required": ["path"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = PathBuf::from(&args.path);

        // Verify directory exists and is a directory
        if !path.exists() {
            return Err(FileError(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }
        if !path.is_dir() {
            return Err(FileError(format!(
                "Path is not a directory: {}",
                path.display()
            )));
        }

        // Build the tree structure
        let mut result = Vec::new();
        let root_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        result.push(root_name);
        build_tree_structure(&path, &mut result, String::from("  "), 1)
            .await
            .map_err(FileError)?;

        Ok(json!({
            "tree": result,
            "path": args.path,
            "entry_count": result.len() - 1, // Excluding the root entry
            "message": format!("Successfully retrieved directory tree for: {}", args.path)
        }))
    }
}

/// Helper function to recursively build the directory tree structure
///
/// # Arguments
///
/// * `dir_path` - Current directory path
/// * `result` - Vector to store tree entries
/// * `prefix` - String prefix for the current level
/// * `depth` - Maximum recursion depth (to prevent excessive output)
///
/// # Returns
///
/// * `Result<(), String>` - Ok on success or error message
pub async fn build_tree_structure(
    dir_path: &Path,
    result: &mut Vec<String>,
    prefix: String,
    depth: usize,
) -> Result<(), String> {
    // Guard against too deep recursion
    if depth > 10 {
        result.push(format!("{}... (max depth reached)", prefix));
        return Ok(());
    }

    // Read directory entries
    let mut entries = match fs::read_dir(dir_path).await {
        Ok(entries) => entries,
        Err(e) => return Err(format!("Failed to read directory: {}", e)),
    };

    // Process all entries
    let mut entry_list = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Skip hidden files and directories (starting with .)
        if name.starts_with('.') {
            continue;
        }

        entry_list.push((path, name));
    }

    // Sort entries: directories first, then files, both alphabetically
    entry_list.sort_by(|(path_a, name_a), (path_b, name_b)| {
        let is_dir_a = path_a.is_dir();
        let is_dir_b = path_b.is_dir();

        if is_dir_a && !is_dir_b {
            std::cmp::Ordering::Less
        } else if !is_dir_a && is_dir_b {
            std::cmp::Ordering::Greater
        } else {
            name_a.cmp(name_b)
        }
    });

    // Process each entry
    for (i, (path, name)) in entry_list.iter().enumerate() {
        let is_last = i == entry_list.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };

        let entry_prefix = format!("{}{}", prefix, connector);
        result.push(format!("{}{}", entry_prefix, name));

        // Recursively process subdirectories
        if path.is_dir() {
            let next_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };

            // Use Box::pin to handle recursion in async functions
            let future = Box::pin(build_tree_structure(path, result, next_prefix, depth + 1));
            future.await?;
        }
    }

    Ok(())
}

impl ToolEmbedding for DirectoryTree {
    type InitError = InitError;
    type Context = ();
    type State = ();

    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(DirectoryTree)
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec![
            "Get a directory tree for a specific path".into(),
            "List files and directories recursively".into(),
            "View folder structure".into(),
        ]
    }

    fn context(&self) -> Self::Context {}
}

// ShowFile Tool
#[derive(Serialize, Deserialize)]
pub struct ShowFile;

impl Tool for ShowFile {
    const NAME: &'static str = "show_file";

    type Error = FileError;
    type Args = OptionalLineRange;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "show_file",
            "description": "View file contents with optional line range - returns text content. Requires prior read permission via request_permission tool.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file"
                    },
                    "start_line": {
                        "type": "integer",
                        "description": "Starting line number (1-based, optional)"
                    },
                    "end_line": {
                        "type": "integer",
                        "description": "Ending line number (inclusive, optional)"
                    }
                },
                "required": ["path"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = PathBuf::from(&args.path);
        let start_line = args.start_line.map(|v| v as usize);
        let end_line = args.end_line.map(|v| v as usize);

        // Read file
        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| FileError(format!("Failed to read file: {}", e)))?;

        // Apply line range if specified
        let filtered_content = if start_line.is_some() || end_line.is_some() {
            let lines: Vec<&str> = content.lines().collect();
            let start = start_line.unwrap_or(1).saturating_sub(1);
            let end = end_line.unwrap_or(lines.len()).min(lines.len());

            if start >= end || start >= lines.len() {
                return Err(FileError(format!(
                    "Invalid line range: {} to {}",
                    start + 1,
                    end
                )));
            }

            lines[start..end].join("\n")
        } else {
            content
        };

        Ok(json!({
            "content": filtered_content,
            "path": args.path,
            "lines": {
                "start": start_line.unwrap_or(1),
                "end": end_line,
                "total": filtered_content.lines().count()
            }
        }))
    }
}

impl ToolEmbedding for ShowFile {
    type InitError = InitError;
    type Context = ();
    type State = ();

    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(ShowFile)
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec![
            "View contents of a file".into(),
            "Read text from a file with option to specify line range".into(),
            "Display file content".into(),
        ]
    }

    fn context(&self) -> Self::Context {}
}

// SearchInFile Tool
#[derive(Serialize, Deserialize)]
pub struct SearchInFile;

impl Tool for SearchInFile {
    const NAME: &'static str = "search_in_file";

    type Error = FileError;
    type Args = SearchParams;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "search_in_file",
            "description": "Search for text patterns or regex in files - returns matching lines with line numbers. Set is_regex=true for regex mode. Requires read permission.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file"
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Search pattern (string or regex)"
                    },
                    "is_regex": {
                        "type": "boolean",
                        "description": "Whether to treat pattern as regex (default: false)",
                        "default": false
                    }
                },
                "required": ["path", "pattern"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = PathBuf::from(&args.path);
        let pattern = &args.pattern;
        let is_regex = args.is_regex.unwrap_or(false);

        // Read file
        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| FileError(format!("Failed to read file: {}", e)))?;

        let lines: Vec<&str> = content.lines().collect();
        let mut matches = Vec::new();

        if is_regex {
            // Compile regex pattern
            let regex = Regex::new(pattern)
                .map_err(|e| FileError(format!("Invalid regex pattern: {}", e)))?;

            // Search for matches
            for (i, line) in lines.iter().enumerate() {
                if regex.is_match(line) {
                    matches.push((i + 1, line.to_string()));
                }
            }
        } else {
            // Simple string search
            for (i, line) in lines.iter().enumerate() {
                if line.contains(pattern) {
                    matches.push((i + 1, line.to_string()));
                }
            }
        }

        Ok(json!({
            "matches": matches.iter().map(|(line_num, content)| {
                json!({
                    "line": line_num,
                    "content": content
                })
            }).collect::<Vec<_>>(),
            "pattern": pattern,
            "is_regex": is_regex,
            "match_count": matches.len(),
            "path": args.path
        }))
    }
}

impl ToolEmbedding for SearchInFile {
    type InitError = InitError;
    type Context = ();
    type State = ();

    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(SearchInFile)
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec![
            "Search for patterns in a file".into(),
            "Find text in files using regex or simple patterns".into(),
            "Locate specific content within files".into(),
        ]
    }

    fn context(&self) -> Self::Context {}
}

// EditFile Tool
#[derive(Serialize, Deserialize)]
pub struct EditFile;

impl Tool for EditFile {
    const NAME: &'static str = "edit_file";

    type Error = FileError;
    type Args = EditParams;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "edit_file",
            "description": "Replace text in files - the old_string must match exactly once in the file. Requires write permission. Use search_in_file first to verify uniqueness.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file"
                    },
                    "old_string": {
                        "type": "string",
                        "description": "Text to be replaced (must be unique in the file)"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "Text to replace with"
                    }
                },
                "required": ["path", "old_string", "new_string"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = PathBuf::from(&args.path);

        // Read file
        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| FileError(format!("Failed to read file: {}", e)))?;

        // Count occurrences of old_string
        let occurrences = content.matches(&args.old_string).count();
        if occurrences == 0 {
            return Err(FileError(format!(
                "String not found in file: {}",
                path.display()
            )));
        } else if occurrences > 1 {
            return Err(FileError(format!(
                "Found {} occurrences of the string in file. Please provide more context to make the match unique.",
                occurrences
            )));
        }

        // Replace string and write back to file
        let new_content = content.replace(&args.old_string, &args.new_string);
        fs::write(&path, new_content)
            .await
            .map_err(|e| FileError(format!("Failed to write file: {}", e)))?;

        Ok(json!({
            "success": true,
            "path": args.path,
            "message": format!("Successfully edited file: {}", args.path)
        }))
    }
}

impl ToolEmbedding for EditFile {
    type InitError = InitError;
    type Context = ();
    type State = ();

    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(EditFile)
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec![
            "Replace text in a file".into(),
            "Edit file content by replacing specific strings".into(),
            "Modify files by substituting text".into(),
        ]
    }

    fn context(&self) -> Self::Context {}
}

// WriteFile Tool
#[derive(Serialize, Deserialize)]
pub struct WriteFile;

impl Tool for WriteFile {
    const NAME: &'static str = "write_file";

    type Error = FileError;
    type Args = WriteParams;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "write_file",
            "description": "Create new files or update existing ones - use append=true to add to file instead of overwriting. Creates files if they don't exist. You should read the contents of the file before writing to it. Requires write permission for the directory.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    },
                    "append": {
                        "type": "boolean",
                        "description": "Whether to append to the file instead of overwriting (default: false)",
                        "default": false
                    }
                },
                "required": ["path", "content"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = PathBuf::from(&args.path);
        let append = args.append.unwrap_or(false);

        // Make sure parent directory exists
        let parent_dir = path
            .parent()
            .ok_or_else(|| FileError("Invalid path: no parent directory".to_string()))?;

        if !parent_dir.exists() {
            return Err(FileError(format!(
                "Directory does not exist: {}",
                parent_dir.display()
            )));
        }

        // Write or append to file
        if append {
            // Create file if it doesn't exist, or append to it
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
                .map_err(|e| FileError(format!("Failed to open file for appending: {}", e)))?;

            file.write_all(args.content.as_bytes())
                .await
                .map_err(|e| FileError(format!("Failed to append to file: {}", e)))?;
        } else {
            // Create or overwrite file
            fs::write(&path, &args.content)
                .await
                .map_err(|e| FileError(format!("Failed to write file: {}", e)))?;
        }

        Ok(json!({
            "success": true,
            "path": args.path,
            "bytes_written": args.content.len(),
            "mode": if append { "append" } else { "overwrite" },
            "message": format!(
                "Successfully {} to file: {}",
                if append { "appended" } else { "wrote" },
                args.path
            )
        }))
    }
}

impl ToolEmbedding for WriteFile {
    type InitError = InitError;
    type Context = ();
    type State = ();

    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(WriteFile)
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec![
            "Create new files or update existing ones".into(),
            "Write content to files with option to append".into(),
            "Save text to filesystem".into(),
        ]
    }

    fn context(&self) -> Self::Context {}
}

// ExecuteShellCommand Tool
#[derive(Serialize, Deserialize)]
pub struct ExecuteShellCommand;

impl Tool for ExecuteShellCommand {
    const NAME: &'static str = "execute_shell_command";

    type Error = FileError;
    type Args = CommandParams;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "execute_shell_command",
            "description": "Run simple shell commands - returns stdout, stderr, and exit code. Limited to safe commands. Requires execute permission first.",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command to execute"
                    },
                    "working_directory": {
                        "type": "string",
                        "description": "Working directory for the command (optional)"
                    }
                },
                "required": ["command"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let command_str = &args.command;

        // Check if command is safe (simple allowlist of safe commands)
        let program = command_str
            .split_whitespace()
            .next()
            .ok_or_else(|| FileError("Empty command".to_string()))?;

        let allowed_commands = ["ls", "cat", "grep", "find", "echo", "pwd", "wc", "head", "tail", "which"];

        if !allowed_commands.contains(&program) {
            return Err(FileError(format!(
                "Command not in allowlist: {}. Only safe, read-only commands are permitted.",
                program
            )));
        }

        // Create command using the detected shell
        let shell = if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "sh"
        };

        let shell_args = if cfg!(target_os = "windows") {
            vec!["/C", command_str]
        } else {
            vec!["-c", command_str]
        };

        // Set up the command
        let mut command = Command::new(shell);
        command.args(&shell_args);

        // Set working directory if specified
        if let Some(ref dir) = args.working_directory {
            let path = PathBuf::from(dir);

            // Verify path exists and is a directory
            if !path.exists() || !path.is_dir() {
                return Err(FileError(format!("Invalid working directory: {}", dir)));
            }

            command.current_dir(path);
        }

        // Execute command
        let output = command
            .output()
            .await
            .map_err(|e| FileError(format!("Failed to execute command: {}", e)))?;

        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
            "command": args.command,
            "working_directory": args.working_directory,
            "success": exit_code == 0
        }))
    }
}

impl ToolEmbedding for ExecuteShellCommand {
    type InitError = InitError;
    type Context = ();
    type State = ();

    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(ExecuteShellCommand)
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec![
            "Execute shell commands".into(),
            "Run system commands with optional working directory".into(),
            "Execute terminal operations".into(),
        ]
    }

    fn context(&self) -> Self::Context {}
}

/// Basic validation to prevent access to system directories
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
