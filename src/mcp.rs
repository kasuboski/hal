//! Model Context Protocol (MCP) server implementation
//!
//! This module provides a secure MCP server that offers file and shell operation capabilities.
//! It implements:
//!
//! - Session-based permission system to maintain permissions throughout user sessions
//! - File operations: view, search, edit, and write files with proper permission checks
//! - Shell operations: execute commands with validation and security checks
//! - Permission management: request and track permissions for directories and commands
//!
//! The implementation balances security with usability by requiring explicit user permission
//! grants while maintaining those permissions throughout the session.

mod code;
mod executor;
mod file_utils;
mod permissions;
mod shell_utils;
mod tools;

use executor::Executor;
pub use permissions::{create_permissions, PermissionsRef, SessionPermissions};
use shell_utils::ShellExecutor;
use std::sync::Arc;
use tokio::sync::Mutex;
pub use tools::{register_tools, tools};

use mcpr::{
    error::MCPError,
    server::{Server, ServerConfig},
    transport::stdio::StdioTransport,
};
use tracing::{info, instrument};

/// Run the MCP server with the given configuration
///
/// This function initializes and starts the MCP server with the provided name and version,
/// setting up all necessary tools and permission management. It:
///
/// 1. Creates a shared permissions object to track allowed directories and commands
/// 2. Configures the server with the provided name and version
/// 3. Registers all tool handlers for file and shell operations
/// 4. Starts the server and begins listening for tool invocations
///
/// # Arguments
///
/// * `name` - The name of the server
/// * `version` - The version string of the server
/// * `transport` - The transport mechanism for communication (StdioTransport)
///
/// # Returns
///
/// * `Result<(), MCPError>` - Ok on successful run, or an MCPError if something fails
#[instrument(skip(transport))]
pub async fn run(name: String, version: String, transport: StdioTransport) -> Result<(), MCPError> {
    info!("Starting HAL MCP server: {} v{}", name, version);

    // Create state containing permissions and executor
    let state = State::new();

    // Configure the server
    let mut server_config = ServerConfig::new()
        .with_name(name.as_str())
        .with_version(version.as_str());

    server_config.tools = tools();

    // Create the server
    let mut server = Server::new(server_config);

    // Register all tool handlers and add tools to config
    register_tools(&mut server, state)?;

    // Start the server
    info!("Server listening for tool invocations...");
    server.serve(transport).await
}

/// State for the MCP server
#[derive(Clone)]
pub struct State {
    permissions: PermissionsRef,
    executor: Arc<dyn Executor + Send + Sync>,
    project_path: Arc<Mutex<Option<String>>>,
}

impl State {
    /// Create a new state with the given permissions
    pub fn new() -> Self {
        let permissions = create_permissions();
        // Create the shell executor
        let executor = Arc::new(ShellExecutor::new(permissions.clone()));

        State {
            permissions,
            executor,
            project_path: Arc::new(Mutex::new(None)),
        }
    }

    /// Get a reference to the permissions
    pub fn permissions(&self) -> PermissionsRef {
        self.permissions.clone()
    }

    /// Get a reference to the executor
    pub fn executor(&self) -> Arc<dyn Executor + Send + Sync> {
        self.executor.clone()
    }

    pub fn project_path(&self) -> Arc<Mutex<Option<String>>> {
        self.project_path.clone()
    }
}
