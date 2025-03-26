use anyhow::Result;
use rig::{
    completion::ToolDefinition,
    tool::{Tool, ToolEmbedding},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

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
        // In a real implementation, this would check permissions and list directory contents
        // Example simplified implementation
        let path = &args.path;
        
        Ok(json!({
            "tree": [path, "  ├── file1.txt", "  ├── file2.txt", "  └── dir1", "      └── nested.txt"],
            "path": path,
            "entry_count": 4,
            "message": format!("Successfully retrieved directory tree for: {}", path)
        }))
    }
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
            "View folder structure".into()
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
        // In a real implementation, this would check permissions and read the file
        // Example simplified implementation
        let content = "This is the content of the file.\nSecond line.\nThird line.";
        let total_lines = content.lines().count();
        
        Ok(json!({
            "content": content,
            "path": args.path,
            "lines": {
                "start": args.start_line.unwrap_or(1),
                "end": args.end_line,
                "total": total_lines
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
            "Display file content".into()
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
        // In a real implementation, this would check permissions and search the file
        // Example simplified implementation
        let matches = [(1, "This line contains the search pattern".to_string()),
            (5, "Another match found here".to_string())];
        
        Ok(json!({
            "matches": matches.iter().map(|(line_num, content)| {
                json!({
                    "line": line_num,
                    "content": content
                })
            }).collect::<Vec<_>>(),
            "pattern": args.pattern,
            "is_regex": args.is_regex.unwrap_or(false),
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
            "Locate specific content within files".into()
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
        // In a real implementation, this would check permissions and edit the file
        // Example simplified implementation
        
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
            "Modify files by substituting text".into()
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
        // In a real implementation, this would check permissions and write to the file
        // Example simplified implementation
        let append = args.append.unwrap_or(false);
        let mode = if append { "append" } else { "overwrite" };
        
        Ok(json!({
            "success": true,
            "path": args.path,
            "bytes_written": args.content.len(),
            "mode": mode,
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
            "Save text to filesystem".into()
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
        // In a real implementation, this would check permissions and execute the command
        // Example simplified implementation
        
        Ok(json!({
            "stdout": "Command executed successfully",
            "stderr": "",
            "exit_code": 0,
            "command": args.command,
            "working_directory": args.working_directory,
            "success": true
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
            "Execute terminal operations".into()
        ]
    }

    fn context(&self) -> Self::Context {}
}
