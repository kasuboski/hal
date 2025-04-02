//! RMCP configuration structures
//!
//! This module provides configuration structures for the RMCP protocol implementation.
//! It defines configurations for different transport methods (SSE, stdio) and server
//! management.

use rmcp::{RoleClient, ServiceExt, service::RunningService};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;

/// Configuration for an MCP server
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServerConfig {
    /// Name of the server
    pub name: String,

    /// Transport configuration (flattened into this struct)
    #[serde(flatten)]
    pub transport: McpServerTransportConfig,
}

/// Transport configuration for MCP servers
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "protocol", rename_all = "lowercase")]
pub enum McpServerTransportConfig {
    /// Server-Sent Events transport
    Sse {
        /// URL for the SSE endpoint
        url: String,
    },

    /// Standard IO transport
    Stdio {
        /// Command to execute
        command: String,

        /// Command line arguments
        #[serde(default)]
        args: Vec<String>,

        /// Environment variables
        #[serde(default)]
        envs: HashMap<String, String>,
    },
}

/// Configuration for multiple MCP servers
#[derive(Debug, Serialize, Deserialize)]
pub struct McpConfig {
    /// List of server configurations
    pub server: Vec<McpServerConfig>,
}

// Implementation for starting server with transport
impl McpServerTransportConfig {
    /// Start a server with the configured transport
    pub async fn start(&self) -> anyhow::Result<RunningService<RoleClient, ()>> {
        let client = match self {
            McpServerTransportConfig::Sse { url } => {
                let transport = rmcp::transport::SseTransport::start(url).await?;
                ().serve(transport).await?
            }
            McpServerTransportConfig::Stdio {
                command,
                args,
                envs,
            } => {
                let transport = rmcp::transport::TokioChildProcess::new(
                    tokio::process::Command::new(command)
                        .args(args)
                        .envs(envs)
                        .stderr(Stdio::null()),
                )?;
                ().serve(transport).await?
            }
        };
        Ok(client)
    }
}

// Implementation for creating manager
impl McpConfig {
    pub async fn read_config(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let config = tokio::fs::read_to_string(path).await?;
        Ok(serde_json::from_str(&config)?)
    }

    /// Create an MCP manager from configuration
    pub async fn create_manager(&self) -> anyhow::Result<crate::mcp::adaptor::McpManager> {
        let mut clients = HashMap::new();
        let mut task_set = tokio::task::JoinSet::<anyhow::Result<_>>::new();

        for server in &self.server {
            let server = server.clone();
            task_set.spawn(async move {
                let client = server.transport.start().await?;
                anyhow::Result::Ok((server.name.clone(), client))
            });
        }

        let start_up_result = task_set.join_all().await;
        for result in start_up_result {
            match result {
                Ok((name, client)) => {
                    clients.insert(name, client);
                }
                Err(e) => {
                    eprintln!("Failed to start server: {:?}", e);
                }
            }
        }

        Ok(crate::mcp::adaptor::McpManager { clients })
    }
}
