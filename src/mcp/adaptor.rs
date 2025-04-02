//! Tool adaptation layer for RMCP

use rig::{
    completion::ToolDefinition,
    tool::{ToolDyn as RigTool, ToolSet},
};
use rmcp::{
    RoleClient,
    model::{CallToolRequestParam, CallToolResult, Tool as McpTool},
    service::{RunningService, ServerSink},
};
use std::{collections::HashMap, future::Future, sync::Arc};
use tracing::{Instrument, debug, error, info, warn}; // Added tracing imports

/// Adapter that makes RMCP tools compatible with RIG
#[derive(Clone)]
pub struct McpToolAdaptor {
    /// The underlying RMCP tool definition
    pub tool: Arc<McpTool>, // Use Arc for cheaper cloning

    /// The server sink to call the tool on
    pub server: ServerSink, // ServerSink is already Clone
}

// Implement the RIG ToolDyn trait for McpToolAdaptor
#[allow(refining_impl_trait)]
impl RigTool for McpToolAdaptor {
    fn name(&self) -> String {
        self.tool.name.to_string()
    }

    // definition() now directly creates the ToolDefinition from the Arc<McpTool>
    fn definition(
        &self,
        _prompt: String,
    ) -> std::pin::Pin<Box<dyn Future<Output = rig::completion::ToolDefinition> + Send + Sync + '_>>
    {
        Box::pin(std::future::ready(rig::completion::ToolDefinition {
            name: self.name(),
            description: self.tool.description.to_owned().to_string(),
            parameters: self.tool.schema_as_json_value(),
        }))
    }

    fn call(
        &self,
        args: String,
    ) -> std::pin::Pin<
        Box<dyn Future<Output = Result<String, rig::tool::ToolError>> + Send + Sync + '_>,
    > {
        let server = self.server.clone();
        Box::pin(async move {
            debug!(tool = %self.name(), args = ?args, "Calling RMCP tool via adaptor");
            let arguments = serde_json::from_str(&args).map_err(rig::tool::ToolError::JsonError)?;
            let call_mcp_tool_result = server
                .call_tool(CallToolRequestParam {
                    name: self.tool.name.clone(),
                    arguments,
                })
                .instrument(tracing::info_span!("mcp_call_tool", tool = %self.name(), args = ?args))
                .await
                .map_err(|e| rig::tool::ToolError::ToolCallError(Box::new(e)))?;

            Ok(convert_mcp_call_tool_result_to_string(call_mcp_tool_result))
        })
    }
}

/// Manager for RMCP clients
pub struct McpManager {
    pub clients: HashMap<String, RunningService<RoleClient, ()>>,
}

impl McpManager {
    /// Get a set of all tools and their definitions from all clients.
    /// Returns a tuple: (Combined ToolSet, Vec of all ToolDefinitions).
    pub async fn get_tool_set_and_defs(&self) -> anyhow::Result<(ToolSet, Vec<ToolDefinition>)> {
        let mut combined_tool_set = ToolSet::default();
        let mut all_definitions = Vec::new();
        let mut tasks = Vec::new();

        info!(
            num_clients = self.clients.len(),
            "Fetching toolsets and definitions from MCP clients"
        );

        for (server_name, client) in &self.clients {
            let server_sink = client.peer().clone();
            let server_name = server_name.clone();

            tasks.push(tokio::spawn(async move {
                debug!(server_name = %server_name, "Requesting tools list and definitions");
                // Call the modified function that returns both
                match get_tool_set_and_defs_from_server(server_sink).await {
                    Ok((server_tool_set, server_definitions)) => {
                        Ok((server_name, server_tool_set, server_definitions)) // Return tuple
                    }
                    Err(e) => {
                        error!(server_name = %server_name, error = %e, "Failed to get tool set and definitions from server");
                        Err(anyhow::anyhow!("Failed to get tools/defs from {}: {}", server_name, e))
                    }
                }
            }));
        }

        for task in tasks {
            match task.await {
                Ok(Ok((_server_name, server_tool_set, server_definitions))) => {
                    // Merge the ToolSet and extend the definitions Vec
                    combined_tool_set.add_tools(server_tool_set);
                    all_definitions.extend(server_definitions);
                }
                Ok(Err(e)) => {
                    warn!("Error gathering tools/defs from one server: {}", e);
                    // Decide if this should be a fatal error
                    // return Err(e);
                }
                Err(join_error) => {
                    error!("Tokio JoinError while fetching tools/defs: {}", join_error);
                    return Err(join_error.into());
                }
            }
        }

        info!(
            total_defs = all_definitions.len(),
            "Finished gathering tools and definitions"
        );
        if all_definitions.is_empty() && !self.clients.is_empty() {
            warn!("No tools were found on any connected MCP servers.");
        }

        Ok((combined_tool_set, all_definitions))
    }
}

/// Convert an RMCP tool result to a string for RIG compatibility.
pub fn convert_mcp_call_tool_result_to_string(result: CallToolResult) -> String {
    if result.content.len() == 1 {
        if let Some(first_content) = result.content.first() {
            if let rmcp::model::RawContent::Text(text_content) = &first_content.raw {
                return text_content.text.clone();
            }
        }
    }
    serde_json::to_string(&result).unwrap_or_else(|e| {
        error!(error = %e, "Failed to serialize CallToolResult");
        format!(
            "{{\"error\": \"Failed to serialize RMCP tool result: {}\"}}",
            e
        )
    })
}

/// Fetches the ToolSet and ToolDefinitions from a single RMCP server sink.
async fn get_tool_set_and_defs_from_server(
    server: ServerSink,
) -> anyhow::Result<(ToolSet, Vec<ToolDefinition>)> {
    let mcp_tools = server.list_all_tools().await?; // Handles pagination
    let mut tool_set = ToolSet::default();
    let mut definitions = Vec::with_capacity(mcp_tools.len());

    for tool in mcp_tools {
        debug!(tool_name = %tool.name, "Adapting MCP tool and generating definition");
        let tool_arc = Arc::new(tool); // Create Arc once

        // Create the adaptor
        let adaptor = McpToolAdaptor {
            tool: tool_arc.clone(), // Clone Arc for adaptor
            server: server.clone(),
        };

        // Create the definition directly from the Arc<McpTool>
        let definition = ToolDefinition {
            name: tool_arc.name.to_string(),
            description: tool_arc.description.to_owned().to_string(),
            parameters: tool_arc.schema_as_json_value(), // Clones underlying data
        };

        tool_set.add_tool(adaptor); // Add the RIG-compatible adaptor
        definitions.push(definition); // Add the definition to the list
    }

    Ok((tool_set, definitions))
}
