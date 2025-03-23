mod permissions;
mod file_utils;
mod shell_utils;
mod tools;

pub use permissions::{SessionPermissions, PermissionsRef, create_permissions};
pub use tools::register_tools;

use mcpr::{
    error::MCPError,
    server::{Server, ServerConfig},
    transport::stdio::StdioTransport,
};
use tracing::{info, instrument};

/// Run the MCP server with the given configuration
#[instrument(skip(transport))]
pub async fn run(name: String, version: String, transport: StdioTransport) -> Result<(), MCPError> {
    info!("Starting HAL MCP server: {} v{}", name, version);
    
    // Create shared permissions
    let permissions = create_permissions();
    
    // Configure the server
    let server_config = ServerConfig::new()
        .with_name(name.as_str())
        .with_version(version.as_str());
    
    // Create the server
    let mut server = Server::new(server_config);
    
    // Register all tool handlers
    register_tools(&mut server, permissions.clone())?;
    
    // Start the server
    info!("Server listening for tool invocations...");
    server.serve(transport).await
}
