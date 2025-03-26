use anyhow::Result;
use rig::{
    completion::ToolDefinition,
    tool::{Tool, ToolEmbedding},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

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
        // In a real implementation, this would update permissions in a permission management system
        let message = match args.operation.as_str() {
            "read" => format!("Read permission granted for directory: {}", args.path),
            "write" => format!("Write permission granted for directory: {}", args.path),
            "execute" => {
                let program = args.path.split_whitespace().next().unwrap_or("");
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
            "Grant execution permission for a command".into()
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

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        // In a real implementation, this would log the thought
        // Example simplified implementation
        
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
            "Cache information for later use".into()
        ]
    }

    fn context(&self) -> Self::Context {}
}
