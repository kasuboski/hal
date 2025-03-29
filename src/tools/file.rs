use anyhow::Result;
use regex::Regex;
use rig::{
    completion::ToolDefinition,
    tool::{Tool, ToolEmbedding},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;

// Importing from the new locations within the tools module
use super::error::{FileError, InitError};
use super::permissions::basic_path_validation;
use super::shared::{Executor, PermissionsRef, State};

// Parameter structs remain the same
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
#[derive(Serialize, Deserialize, Clone)]
pub struct DirectoryTree {
    #[serde(skip)]
    permissions: PermissionsRef,
}

impl DirectoryTree {
    /// Creates a new DirectoryTree tool with the given state
    pub fn new(state: crate::tools::shared::State) -> Self {
        Self {
            permissions: state.permissions,
        }
    }
}

impl Tool for DirectoryTree {
    const NAME: &'static str = "directory_tree";

    type Error = FileError;
    type Args = super::project::PathParam; // Assuming PathParam is still relevant here
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "directory_tree",
            // Updated description
            "description": "Get a directory tree given a path. Returns a list of directories and files. Requires read permission for the path and its subdirectories. Use request_permission first.",
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

        // --- Validation ---
        basic_path_validation(&path).map_err(FileError)?;

        // Verify path exists and is a directory (basic checks)
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

        // --- Permission Check (Root Directory) ---
        {
            let perms = self.permissions.lock().await;
            if !perms.can_read(&path) {
                return Err(FileError(format!(
                    "Read permission denied for path: {}. Use request_permission first.",
                    path.display()
                )));
            }
        } // Lock released

        // Build the tree structure
        let mut result = Vec::new();
        let root_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        result.push(root_name);
        // Pass permissions to the helper function
        build_tree_structure(&path, &self.permissions, &mut result, String::from("  "), 1)
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
/// Now takes PermissionsRef to check before reading directories.
///
/// # Arguments
///
/// * `dir_path` - Current directory path
/// * `permissions` - Shared permissions reference
/// * `result` - Vector to store tree entries
/// * `prefix` - String prefix for the current level
/// * `depth` - Maximum recursion depth (to prevent excessive output)
///
/// # Returns
///
/// * `Result<(), String>` - Ok on success or error message
pub async fn build_tree_structure(
    dir_path: &Path,
    permissions: &PermissionsRef, // Added permissions parameter
    result: &mut Vec<String>,
    prefix: String,
    depth: usize,
) -> Result<(), String> {
    // Guard against too deep recursion
    if depth > 10 {
        result.push(format!("{}... (max depth reached)", prefix));
        return Ok(());
    }

    // --- Permission Check (Current Directory before reading) ---
    // Note: We already checked the root dir in `call`. This checks subdirs.
    // No need to lock again if we are careful, but locking is safer.
    {
        let perms = permissions.lock().await;
        if !perms.can_read(dir_path) {
            // Don't return error, just indicate restricted access in the tree
            result.push(format!("{} [Permission Denied]", prefix));
            return Ok(());
        }
    } // Lock released

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

        // Skip common build/dependency folders
        if name == "target" || name == "node_modules" {
            result.push(format!("{}{} [Skipped]", prefix, name));
            continue;
        }

        entry_list.push((path, name));
    }

    // Sort entries: directories first, then files, both alphabetically
    entry_list.sort_by(|(path_a, name_a), (path_b, name_b)| {
        // Use metadata_async for checking if dir, handle errors
        let is_dir_a = path_a.is_dir(); // Keep sync check for sorting simplicity if possible
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
        // Check permission *before* recursing
        let is_dir = path.is_dir(); // Re-check or use sorted info
        if is_dir {
            // No need to lock again if we pass the Arc down
            let next_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };

            // Pass permissions down
            let future = Box::pin(build_tree_structure(
                path,
                permissions,
                result,
                next_prefix,
                depth + 1,
            ));
            future.await?;
        }
    }

    Ok(())
}

impl ToolEmbedding for DirectoryTree {
    type InitError = InitError;
    type Context = ();
    type State = State; // Use shared State

    fn init(state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(DirectoryTree {
            permissions: state.permissions.clone(), // Clone Arc
        })
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
#[derive(Serialize, Deserialize, Clone)]
pub struct ShowFile {
    #[serde(skip)]
    permissions: PermissionsRef,
}

impl ShowFile {
    /// Creates a new ShowFile tool with the given state
    pub fn new(state: crate::tools::shared::State) -> Self {
        Self {
            permissions: state.permissions,
        }
    }
}

impl Tool for ShowFile {
    const NAME: &'static str = "show_file";

    type Error = FileError;
    type Args = OptionalLineRange;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "show_file",
            // Updated description
            "description": "View file contents with optional line range - returns text content. Requires read permission for the file. Use request_permission first.",
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

        // --- Validation ---
        basic_path_validation(&path).map_err(FileError)?;

        // --- Permission Check ---
        {
            let perms = self.permissions.lock().await;
            if !perms.can_read(&path) {
                return Err(FileError(format!(
                    "Read permission denied for path: {}. Use request_permission first.",
                    path.display()
                )));
            }
        } // Lock released

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
                // Use saturating_add for 1-based display
                let display_start = start.saturating_add(1);
                return Err(FileError(format!(
                    "Invalid line range: {} to {} (file has {} lines)",
                    display_start,
                    end,
                    lines.len()
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
                "end": end_line.unwrap_or_else(|| filtered_content.lines().count()), // Calculate end if not provided
                "total_in_range": filtered_content.lines().count() // Lines in the returned content
            }
        }))
    }
}

impl ToolEmbedding for ShowFile {
    type InitError = InitError;
    type Context = ();
    type State = State; // Use shared State

    fn init(state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(ShowFile {
            permissions: state.permissions.clone(), // Clone Arc
        })
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
#[derive(Serialize, Deserialize, Clone)]
pub struct SearchInFile {
    #[serde(skip)]
    permissions: PermissionsRef,
}

impl SearchInFile {
    /// Creates a new SearchInFile tool with the given state
    pub fn new(state: crate::tools::shared::State) -> Self {
        Self {
            permissions: state.permissions,
        }
    }
}

impl Tool for SearchInFile {
    const NAME: &'static str = "search_in_file";

    type Error = FileError;
    type Args = SearchParams;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "search_in_file",
            // Updated description
            "description": "Search for text patterns or regex in files - returns matching lines with line numbers. Set is_regex=true for regex mode. Requires read permission for the file. Use request_permission first.",
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

        // --- Validation ---
        basic_path_validation(&path).map_err(FileError)?;

        // --- Permission Check ---
        {
            let perms = self.permissions.lock().await;
            if !perms.can_read(&path) {
                return Err(FileError(format!(
                    "Read permission denied for path: {}. Use request_permission first.",
                    path.display()
                )));
            }
        } // Lock released

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
                    matches.push((i + 1, line.to_string())); // 1-based line number
                }
            }
        } else {
            // Simple string search
            for (i, line) in lines.iter().enumerate() {
                if line.contains(pattern) {
                    matches.push((i + 1, line.to_string())); // 1-based line number
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
    type State = State; // Use shared State

    fn init(state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(SearchInFile {
            permissions: state.permissions.clone(), // Clone Arc
        })
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
#[derive(Serialize, Deserialize, Clone)]
pub struct EditFile {
    #[serde(skip)]
    permissions: PermissionsRef,
}

impl EditFile {
    /// Creates a new EditFile tool with the given state
    pub fn new(state: crate::tools::shared::State) -> Self {
        Self {
            permissions: state.permissions,
        }
    }
}

impl Tool for EditFile {
    const NAME: &'static str = "edit_file";

    type Error = FileError;
    type Args = EditParams;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "edit_file",
            // Updated description
            "description": "Replace text in files - the old_string must match exactly once in the file. Requires write permission for the file. Use request_permission first. Use search_in_file first to verify uniqueness.",
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

        // --- Validation ---
        basic_path_validation(&path).map_err(FileError)?;

        // --- Permission Check ---
        // Check write permission for the file itself
        {
            let perms = self.permissions.lock().await;
            if !perms.can_write(&path) {
                return Err(FileError(format!(
                    "Write permission denied for path: {}. Use request_permission first.",
                    path.display()
                )));
            }
        } // Lock released

        // Read file (requires implicit read permission granted by write)
        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| FileError(format!("Failed to read file: {}", e)))?;

        // Count occurrences of old_string
        let occurrences = content.matches(&args.old_string).count();
        if occurrences == 0 {
            return Err(FileError(format!(
                "String '{}' not found in file: {}",
                args.old_string,
                path.display()
            )));
        } else if occurrences > 1 {
            return Err(FileError(format!(
                "Found {} occurrences of the string '{}' in file. Please provide more context or a unique string to replace.",
                occurrences,
                args.old_string
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
    type State = State; // Use shared State

    fn init(state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(EditFile {
            permissions: state.permissions.clone(), // Clone Arc
        })
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
#[derive(Serialize, Deserialize, Clone)]
pub struct WriteFile {
    #[serde(skip)]
    permissions: PermissionsRef,
}

impl WriteFile {
    /// Creates a new WriteFile tool with the given state
    pub fn new(state: crate::tools::shared::State) -> Self {
        Self {
            permissions: state.permissions,
        }
    }
}

impl Tool for WriteFile {
    const NAME: &'static str = "write_file";

    type Error = FileError;
    type Args = WriteParams;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "write_file",
            // Updated description
            "description": "Create new files or update existing ones - use append=true to add to file instead of overwriting. Creates files if they don't exist. Requires write permission for the directory containing the file. Use request_permission first. You should consider reading the file first if overwriting.",
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

        // --- Validation ---
        basic_path_validation(&path).map_err(FileError)?;

        // Get parent directory for permission check
        let parent_dir = path
            .parent()
            .ok_or_else(|| FileError("Invalid path: no parent directory".to_string()))?;

        // --- Permission Check (Parent Directory) ---
        {
            let perms = self.permissions.lock().await;
            if !perms.can_write(parent_dir) {
                // Check parent dir
                return Err(FileError(format!(
                    "Write permission denied for directory: {}. Use request_permission first.",
                    parent_dir.display()
                )));
            }
        } // Lock released

        // Make sure parent directory exists (filesystem check)
        if !parent_dir.exists() {
            // Attempt to create the directory? Or return error? Let's return error for now.
            // Consider adding a `create_directory` tool or an option here later.
            return Err(FileError(format!(
                "Parent directory does not exist: {}",
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
    type State = State; // Use shared State

    fn init(state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(WriteFile {
            permissions: state.permissions.clone(), // Clone Arc
        })
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
#[derive(Clone)]
pub struct ExecuteShellCommand {
    // PermissionsRef is not directly used here, but held by the executor
    // permissions: PermissionsRef,
    executor: Arc<dyn Executor + Send + Sync>,
}

impl ExecuteShellCommand {
    /// Creates a new ExecuteShellCommand tool with the given state
    pub fn new(state: crate::tools::shared::State) -> Self {
        Self {
            // permissions: state.permissions, // Not needed directly
            executor: state.executor,
        }
    }
}

// --- Serialization/Deserialization for ExecuteShellCommand ---
// These are tricky because the executor is a trait object.
// We might need to skip serialization or handle it carefully if these tools
// need to be serialized/deserialized themselves (e.g., for agent state saving).
// For now, assuming they are constructed via `init` and not directly serialized.
// If serialization is needed, we'd likely need a way to reconstruct the executor.
// Let's keep the previous placeholder Serialize/Deserialize which relies on default State.

impl Serialize for ExecuteShellCommand {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let state = serializer.serialize_struct("ExecuteShellCommand", 0)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ExecuteShellCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ExecuteShellCommandVisitor;

        impl<'de> serde::de::Visitor<'de> for ExecuteShellCommandVisitor {
            type Value = ExecuteShellCommand;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct ExecuteShellCommand")
            }

            // Deserialize as an empty map/struct and reconstruct using default state
            fn visit_map<V>(self, _map: V) -> Result<ExecuteShellCommand, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let default_state = crate::tools::shared::State::default();
                Ok(ExecuteShellCommand::new(default_state))
            }

            // Also handle unit struct deserialization if needed
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let default_state = crate::tools::shared::State::default();
                Ok(ExecuteShellCommand::new(default_state))
            }
        }

        // Allow deserializing from an empty map or unit struct representation
        deserializer.deserialize_any(ExecuteShellCommandVisitor)
        // Or specifically: deserializer.deserialize_struct("ExecuteShellCommand", &[], ExecuteShellCommandVisitor)
    }
}
// --- End Serialization/Deserialization ---

impl Tool for ExecuteShellCommand {
    const NAME: &'static str = "execute_shell_command";

    type Error = FileError;
    type Args = CommandParams;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "execute_shell_command",
            // Updated description
            "description": "Run simple shell commands - returns stdout, stderr, and exit code. Requires execute permission for the command and read permission for the working directory (if specified). Use request_permission first.",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command to execute"
                    },
                    "working_directory": {
                        "type": "string",
                        "description": "Working directory for the command (optional, requires read permission)"
                    }
                },
                "required": ["command"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let command_str = args.command.clone();
        let working_dir_opt = args.working_directory.clone(); // Clone for JSON output
        let working_dir_path = working_dir_opt.map(PathBuf::from);

        // --- Validation (Working Directory Path Only) ---
        if let Some(ref wd) = working_dir_path {
            basic_path_validation(wd).map_err(FileError)?;
            // The executor will handle the read permission check for the WD
        }

        // Use the executor from self - it handles command + WD read permissions internally
        match self
            .executor
            .execute(command_str, working_dir_path.as_deref())
            .await
        {
            Ok(result) => Ok(json!({
                "stdout": result.stdout,
                "stderr": result.stderr,
                "exit_code": result.exit_code,
                "command": args.command,
                "working_directory": args.working_directory, // Use the optional string from args
                "success": result.exit_code == 0
            })),
            // Map the executor's Box<dyn Error> to FileError
            Err(e) => Err(FileError(format!("Command execution failed: {}", e))),
        }
    }
}

impl ToolEmbedding for ExecuteShellCommand {
    type InitError = InitError;
    type Context = ();
    type State = State; // Use shared State

    fn init(state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(ExecuteShellCommand {
            // permissions: state.permissions.clone(), // No longer needed directly
            executor: state.executor.clone(), // Clone Arc for executor
        })
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
