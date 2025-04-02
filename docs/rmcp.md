# RMCP Implementation Guide

## Overview

This document provides guidance on implementing tools using the Rust Model Context Protocol (RMCP) framework. RMCP offers an efficient way to expose functionality to LLM-based clients through a standard protocol.

## Core Components

RMCP has several core components:
- **Tools**: Functions that can be called by LLM clients
- **ServerHandler**: Interface for handling server requests
- **Transport**: Communication layer (stdio, HTTP, etc.)
- **Content**: Structured data returned from tools

## Tool Implementation Patterns

### Attribute Macro Pattern (Recommended)

RMCP provides a convenient attribute macro system for defining tools:

```rust
use rmcp::{ServerHandler, tool, model::*, Error};
use schemars;

#[derive(Clone)]
pub struct MyTools {
    // State for the server and tools
    state_data: Arc<Mutex<SomeData>>,
}

#[tool(tool_box)]
impl MyTools {
    pub fn new() -> Self {
        Self {
            state_data: Arc::new(Mutex::new(SomeData::default())),
        }
    }
    
    #[tool(description = "Tool that does something useful")]
    async fn my_tool(&self, 
        #[tool(param)]
        #[schemars(description = "Description of the parameter")]
        param_name: String,
        
        #[tool(param)]
        #[schemars(description = "Another parameter")]
        other_param: Option<i32>
    ) -> Result<CallToolResult, Error> {
        // Tool implementation
        Ok(CallToolResult::success(vec![Content::text("Result data")]))
    }
    
    #[tool(description = "Another tool example")]
    fn simple_tool(&self, 
        #[tool(param)] input: String
    ) -> String {
        // For simple returns, you can just return a String
        format!("Processed: {}", input)
    }
    
    #[tool(description = "Tool using a parameter struct")]
    fn struct_param_tool(&self,
        #[tool(aggr)] params: MyParams
    ) -> Result<CallToolResult, Error> {
        let result = params.a + params.b;
        Ok(CallToolResult::success(vec![Content::text(result.to_string())]))
    }
}

// Parameter struct for aggregating multiple parameters
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MyParams {
    #[schemars(description = "First number")]
    pub a: i32,
    
    #[schemars(description = "Second number")]
    pub b: i32,
}

// Implement ServerHandler on the same struct
#[tool(tool_box)]
impl ServerHandler for MyTools {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "My RMCP Server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("This server provides various tools".to_string()),
        }
    }
    
    // You don't need to implement list_tools or call_tool with the tool_box attribute
    // They are automatically implemented based on your tool definitions
}
```

### Server Initialization

Starting the RMCP server is straightforward:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an instance of your tools
    let tools = MyTools::new();
    
    // Create a transport (e.g., stdio)
    let transport = rmcp::transport::stdio();
    
    // Start the server
    let service = tools.serve(transport).await?;
    
    // Wait for the server to complete
    service.waiting().await?;
    
    Ok(())
}
```

## RMCP Attribute Reference

### Tool Definition Attributes

- **`#[tool(tool_box)]`**: Applied to an `impl` block to indicate it contains tool implementations
- **`#[tool(description = "...")]`**: Applied to methods to define tools with descriptions
- **`#[tool(param)]`**: Applied to method parameters to define tool inputs
- **`#[tool(aggr)]`**: Used for struct parameters that group multiple inputs
- **`#[schemars(description = "...")]`**: Used to document parameter descriptions

### Return Types

Tools can return:

1. **`Result<CallToolResult, Error>`**: Full control over response format
2. **`String`**: Automatically converted to a text Content
3. **Other types**: Must implement serde::Serialize, converted to JSON

### Error Handling

RMCP provides built-in error creation functions:

```rust
// Various error creation patterns
Error::invalid_request("Invalid parameter", None)
Error::internal_error("Something went wrong", Some(json!({ "details": error_info })))
Error::resource_not_found("Resource not found", None)
```

## Parameter Handling

### Basic Parameters

Simple parameters can be directly annotated:

```rust
#[tool(description = "Simple parameter example")]
fn example(&self,
    #[tool(param)]
    #[schemars(description = "A string parameter")]
    text: String,
    
    #[tool(param)]
    #[schemars(description = "Optional number", minimum = 1, maximum = 100)]
    number: Option<i32>
) -> String {
    // Implementation
}
```

### Struct Parameters

For multiple related parameters, use the aggregate pattern:

```rust
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct QueryParams {
    #[schemars(description = "Search query string")]
    query: String,
    
    #[schemars(description = "Maximum results to return", default = 10, minimum = 1, maximum = 100)]
    limit: Option<usize>,
    
    #[schemars(description = "Filter by category")]
    category: Option<String>
}

#[tool(description = "Search with multiple parameters")]
fn search(&self, #[tool(aggr)] params: QueryParams) -> Result<CallToolResult, Error> {
    // Implementation using params.query, params.limit, etc.
}
```

## Content Types

RMCP supports various content types in responses:

```rust
// Text content
Content::text("Plain text result")

// JSON content
Content::json(json!({ "key": "value", "data": [1, 2, 3] }))

// Binary content
Content::binary(vec![0, 1, 2, 3, 4], "application/octet-stream")

// Multiple content items
CallToolResult::success(vec![
    Content::text("Text explanation"),
    Content::json(json!({ "data": results }))
])
```

## Advanced Features

### State Management

Maintain state between tool calls using shared resources:

```rust
#[derive(Clone)]
pub struct MyTools {
    counter: Arc<Mutex<i32>>,
    cache: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

#[tool(tool_box)]
impl MyTools {
    #[tool(description = "Increment counter")]
    async fn increment(&self) -> String {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        counter.to_string()
    }
    
    #[tool(description = "Get cached data")]
    async fn get_cached(&self, #[tool(param)] key: String) -> Result<CallToolResult, Error> {
        let cache = self.cache.lock().await;
        match cache.get(&key) {
            Some(data) => Ok(CallToolResult::success(vec![Content::json(json!(data))])),
            None => Err(Error::resource_not_found("Cache key not found", None))
        }
    }
}
```

### Resource Handling

RMCP supports resource handling for files and other data sources:

```rust
#[tool(tool_box)]
impl ServerHandler for MyTools {
    // Implement resource listing
    async fn list_resources(
        &self,
        _request: PaginatedRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, Error> {
        Ok(ListResourcesResult {
            resources: vec![
                RawResource::new("file:///path/to/file.txt", "Example File").no_annotation(),
                // More resources...
            ],
            next_cursor: None,
        })
    }
    
    // Implement resource reading
    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, Error> {
        match uri.as_str() {
            "file:///path/to/file.txt" => {
                let content = std::fs::read_to_string("/path/to/file.txt")
                    .map_err(|e| Error::internal_error(format!("Failed to read file: {}", e), None))?;
                
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(content, uri)],
                })
            },
            _ => Err(Error::resource_not_found("Resource not found", None)),
        }
    }
}
```

## Best Practices

1. **Tool Organization**: Group related tools in the same impl block
2. **Descriptive Names**: Use clear, action-oriented names for tools
3. **Detailed Descriptions**: Provide thorough descriptions for tools and parameters
4. **Parameter Validation**: Validate parameters early to provide clear error messages
5. **Error Handling**: Return appropriate error types with helpful messages
6. **State Management**: Use thread-safe containers for shared state
7. **Async When Needed**: Use `async` for I/O operations, database access, etc.
8. **Testing**: Write unit tests for individual tools and integration tests for the server

## Common Pitfalls

1. **Missing Clone**: The tool handler struct must implement Clone
2. **Incorrect Parameter Annotations**: Ensure each parameter has the correct attributes
3. **Complex Return Types**: Prefer simple return types or explicit CallToolResult
4. **Blocking Operations**: Avoid blocking operations in async contexts
5. **Error Propagation**: Use `?` operator with appropriate error conversion

## References

- [RMCP Crate Documentation](https://docs.rs/rmcp/)
- [RMCP Tool Macros](https://docs.rs/rmcp/latest/rmcp/tool/index.html)
- [Schemars Documentation](https://docs.rs/schemars/)
