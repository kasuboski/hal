use std::collections::HashMap;

use mcpr::{
    error::MCPError,
    schema::ToolInputSchema,
    server::{Server, ServerConfig},
    transport::stdio::StdioTransport,
    Tool,
};
use serde_json::{json, Value};
use tracing::{info, instrument};

#[instrument(skip(transport))]
pub async fn run(name: String, version: String, transport: StdioTransport) -> Result<(), MCPError> {
    // Create an echo tool
    let echo_tool = Tool {
        name: "echo".to_string(),
        description: Some("Echoes back the input".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(
                [(
                    "message".to_string(),
                    json!({
                        "type": "string",
                        "description": "The message to echo"
                    }),
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
            required: Some(vec!["message".to_string()]),
        },
    };

    // Create a hello tool
    let hello_tool = Tool {
        name: "hello".to_string(),
        description: Some("Says hello to someone".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(
                [(
                    "name".to_string(),
                    json!({
                        "type": "string",
                        "description": "The name to greet"
                    }),
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
            required: Some(vec!["name".to_string()]),
        },
    };

    // Create a search tool
    let search_tool = Tool {
        name: "search".to_string(),
        description: Some("Search previously indexed content".to_string()),
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
    };

    // Configure the server
    let server_config = ServerConfig::new()
        .with_name(name.as_str())
        .with_version(version.as_str())
        .with_tool(echo_tool)
        .with_tool(hello_tool)
        .with_tool(search_tool);

    // Create the server
    let mut server = Server::new(server_config);

    // Register tool handlers
    server.register_tool_handler("echo", |params: serde_json::Value| async move {
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Protocol("Missing message parameter".to_string()))?;

        info!("Echo request: {}", message);

        Ok(json!({
            "result": message
        }))
    })?;

    server.register_tool_handler("hello", |params: Value| async move {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Protocol("Missing name parameter".to_string()))?;

        info!("Hello request for name: {}", name);

        Ok(json!({
            "result": format!("Hello, {}!", name)
        }))
    })?;

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

    // Start the server
    info!("Server listening for tool invocations...");
    server.serve(transport).await
}
