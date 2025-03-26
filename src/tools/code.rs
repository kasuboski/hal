use anyhow::Result;
use rig::{
    completion::ToolDefinition,
    tool::{Tool, ToolEmbedding},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

// Importing common error types from project module
use super::project::{FileError, InitError, PathParam};

// Search query parameter
#[derive(Deserialize)]
pub struct SearchQueryParams {
    pub query: String,
}

// CodeRepoOverview Tool
#[derive(Serialize, Deserialize)]
pub struct CodeRepoOverview;

impl Tool for CodeRepoOverview {
    const NAME: &'static str = "code_repo_overview";

    type Error = FileError;
    type Args = PathParam;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "code_repo_overview",
            "description": "Get an overview of the code repository. Returns a list of files and their contents. This will return a large costly response.",
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
        // In a real implementation, this would analyze the code repository
        // Example simplified implementation

        Ok(json!({
            "overview": "This is a code repository with multiple files and directories",
            "files": 10,
            "path": args.path
        }))
    }
}

impl ToolEmbedding for CodeRepoOverview {
    type InitError = InitError;
    type Context = ();
    type State = ();

    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(CodeRepoOverview)
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec![
            "Get an overview of a code repository".into(),
            "Analyze code files in a directory".into(),
            "Summarize a codebase".into(),
        ]
    }

    fn context(&self) -> Self::Context {}
}

// Search Tool
#[derive(Serialize, Deserialize)]
pub struct Search;

impl Tool for Search {
    const NAME: &'static str = "search";

    type Error = FileError;
    type Args = SearchQueryParams;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "search",
            "description": "Search indexed website content using semantic search - returns relevant text chunks with their sources. Used for retrieving information from previously crawled websites.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    }
                },
                "required": ["query"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        // In a real implementation, this would search the indexed content
        // Example simplified implementation
        let results = vec![
            json!({
                "text": "This is a relevant snippet of text matching the query.",
                "url": "https://example.com/page1",
                "context": "More context about this snippet"
            }),
            json!({
                "text": "Another relevant result for the search.",
                "url": "https://example.com/page2",
                "context": "Additional context about this result"
            }),
        ];

        Ok(json!({
            "results": results
        }))
    }
}

impl ToolEmbedding for Search {
    type InitError = InitError;
    type Context = ();
    type State = ();

    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(Search)
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec![
            "Search indexed website content".into(),
            "Find information in previously crawled data".into(),
            "Query over indexed documents".into(),
        ]
    }

    fn context(&self) -> Self::Context {}
}
