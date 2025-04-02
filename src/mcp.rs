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

// Core modules
pub mod code;
pub mod executor;
pub mod file_utils;
pub mod permissions;
pub mod shell_utils;

// Tool implementation modules
pub mod adaptor;
pub mod config;
pub mod tool_core;
pub mod tool_file;
pub mod tool_search;
pub mod tool_shell;
pub mod tools_rmcp;

use executor::Executor;
pub use permissions::{PermissionsRef, SessionPermissions, create_permissions};
use shell_utils::ShellExecutor;
use std::sync::Arc;
use tokio::sync::Mutex;

use rmcp::{
    RoleServer,
    handler::server::tool::ToolCallContext,
    model::{
        CallToolRequestParam, CallToolResult, ErrorCode, Implementation, ListToolsResult,
        PaginatedRequestParam, ServerCapabilities, ServerInfo,
    },
    serve_server,
    service::RequestContext,
    tool,
    transport::stdio,
};
use tracing::{info, instrument, warn};

/// Run the MCP server with the given configuration
///
/// This function initializes and starts the MCP server with the provided name and version,
/// setting up all necessary tools and permission management. It:
///
/// 1. Creates a shared state containing permissions and executor.
/// 2. Configures the server implementation.
/// 3. Creates a standard I/O transport.
/// 4. Starts the server using the transport and waits for it to complete.
///
/// # Arguments
///
/// * `name` - The name of the server (used for logging/identification).
/// * `version` - The version string of the server (used for logging/identification).
///
/// # Returns
///
/// * `anyhow::Result<()>` - Ok(()) on successful server run and shutdown, or an error if setup or execution fails.
///
/// # Example
///
/// ```no_run
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let name = "My MCP Server".to_string();
/// let version = "0.1.0".to_string();
/// hal::mcp::run(name, version).await?; // Call the actual function
/// # Ok(())
/// # }
/// ```
#[instrument]
pub async fn run(name: String, version: String) -> anyhow::Result<()> {
    info!("Starting HAL MCP server: {} v{}", name, version);

    // Create state containing permissions and executor
    let state = State::new();

    // Create the HAL server with state
    let hal_server = HalServer::new(state);

    // Create stdio transport
    let transport = stdio();

    // Start the server with the main server handler
    // The #[tool] attributes will handle registration
    info!("Server listening for tool invocations...");
    let server = serve_server(hal_server, transport).await?;

    // Wait for server to complete
    server.waiting().await?;

    Ok(())
}

/// Main RMCP Server Handler implementation that delegates to the tool handlers
#[derive(Clone)]
pub struct HalServer {
    state: State,
}

#[tool(tool_box)]
impl HalServer {
    pub fn new(state: State) -> Self {
        Self { state }
    }

    pub fn core_tools(&self) -> tool_core::CoreTools {
        tool_core::CoreTools::new(self.state.permissions(), self.state.project_path())
    }

    fn file_tools(&self) -> tool_file::FileTools {
        tool_file::FileTools::new(self.state.permissions())
    }

    fn shell_tools(&self) -> tool_shell::ShellTools {
        tool_shell::ShellTools::new(
            self.state.executor(),
            self.state.permissions(),
            self.state.project_path(),
        )
    }

    fn search_tools(&self) -> tool_search::SearchTools {
        tool_search::SearchTools::new()
    }
}

// Implement ServerHandler for the main server
impl rmcp::ServerHandler for HalServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
            server_info: Implementation {
                name: "HAL".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(
                "HAL MCP Server provides secure access to file and shell operations with a permissions system. \
                \n\n1. INITIALIZATION: Call the `init` tool first with a project directory path to establish context \
                and receive initial read/write permissions for that directory. \
                \n\n2. PERMISSIONS: Before accessing files or running commands, you must request explicit permissions: \
                \n   - Use `request_permission` with operation='read' and path=<directory> for file reading operations \
                \n   - Use `request_permission` with operation='write' and path=<directory> for file writing operations \
                \n   - Use `request_permission` with operation='execute' and path=<command> for shell command execution \
                \n\n3. FILE OPERATIONS: After permissions are granted, you can use tools like `show_file`, `search_in_file`, \
                `edit_file`, and `write_file`. \
                \n\n4. SHELL OPERATIONS: After execution permission, you can use `execute_shell_command`. \
                \n\nPermissions persist throughout your session once granted. The system enforces security \
                by limiting access to only explicitly permitted directories and commands.".to_string(),
            ),
        }
    }

    fn list_tools(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, rmcp::Error>> + Send + '_ {
        async {
            let mut all_tools = Vec::new();
            all_tools.extend(tool_core::CoreTools::get_tool_box().list());
            all_tools.extend(tool_file::FileTools::get_tool_box().list());
            all_tools.extend(tool_shell::ShellTools::get_tool_box().list());
            all_tools.extend(tool_search::SearchTools::get_tool_box().list());
            Ok(ListToolsResult {
                tools: all_tools,
                next_cursor: None,
            })
        }
    }

    // TODO: there seems like there should a better way to do this
    fn call_tool(
        &self,
        request_params: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, rmcp::Error>> + Send + '_ {
        async move {
            let tool_name = request_params.name.as_ref();
            info!("Received tool call for: {}", tool_name);

            // Use get_tool_box() for checking and calling
            let result = if tool_core::CoreTools::get_tool_box()
                .map
                .contains_key(tool_name)
            {
                info!("Delegating to CoreTools...");
                let core_tools_instance = self.core_tools();
                let core_context =
                    ToolCallContext::new(&core_tools_instance, request_params, context);
                // Call via the public accessor
                tool_core::CoreTools::get_tool_box()
                    .call(core_context)
                    .await
            } else if tool_file::FileTools::get_tool_box()
                .map
                .contains_key(tool_name)
            {
                info!("Delegating to FileTools...");
                let file_tools_instance = self.file_tools();
                let file_context =
                    ToolCallContext::new(&file_tools_instance, request_params, context);
                tool_file::FileTools::get_tool_box()
                    .call(file_context)
                    .await // Use get_tool_box()
            } else if tool_shell::ShellTools::get_tool_box()
                .map
                .contains_key(tool_name)
            {
                info!("Delegating to ShellTools...");
                let shell_tools_instance = self.shell_tools();
                let shell_context =
                    ToolCallContext::new(&shell_tools_instance, request_params, context);
                tool_shell::ShellTools::get_tool_box()
                    .call(shell_context)
                    .await // Use get_tool_box()
            } else if tool_search::SearchTools::get_tool_box()
                .map
                .contains_key(tool_name)
            {
                info!("Delegating to SearchTools...");
                let search_tools_instance = self.search_tools();
                let search_context =
                    ToolCallContext::new(&search_tools_instance, request_params, context);
                tool_search::SearchTools::get_tool_box()
                    .call(search_context)
                    .await // Use get_tool_box()
            } else {
                warn!("Tool not found: {}", tool_name);
                // Using specific error recommended
                Err(rmcp::Error::new(
                    ErrorCode::METHOD_NOT_FOUND,
                    format!("Tool '{}' not found", tool_name),
                    None,
                ))
            };

            // ... (logging success/failure) ...
            result
        }
    }
}

/// State for the MCP server
#[derive(Clone)]
pub struct State {
    permissions: PermissionsRef,
    executor: Arc<dyn Executor + Send + Sync>,
    project_path: Arc<Mutex<Option<String>>>,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
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
