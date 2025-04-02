//! RMCP tools implementation coordinator
//!
//! This module coordinates all RMCP tool implementations.

use crate::mcp::State;
use crate::mcp::tool_core::CoreTools;
use crate::mcp::tool_file::FileTools;
use crate::mcp::tool_search::SearchTools;
use crate::mcp::tool_shell::ShellTools;

/// Create and configure all tool handlers for RMCP server
pub fn create_tools(state: State) -> (CoreTools, FileTools, ShellTools, SearchTools) {
    // Core tools
    let core_tools = CoreTools::new(state.permissions(), state.project_path());

    // File tools
    let file_tools = FileTools::new(state.permissions());

    // Shell tools
    let shell_tools = ShellTools::new(state.executor(), state.permissions(), state.project_path());

    // Search tools
    let search_tools = SearchTools::new();

    (core_tools, file_tools, shell_tools, search_tools)
}
