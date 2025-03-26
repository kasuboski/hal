//! Tool implementations for Hal using the Rig library
//!
//! This module contains implementations of various tools that can be used
//! with LLM agents. The tools are organized into categories:
//!
//! - Project tools: For project setup, permissions, and general operations
//! - File tools: For file system operations like reading, writing, and searching
//! - Code tools: For code analysis and search operations

pub mod code;
pub mod file;
pub mod project;

use rig::tool::ToolSet;

/// Get all tools packaged in a ToolSet
///
/// Creates and returns a ToolSet with all the tools implemented by HAL.
/// This can be used to initialize an agent with static tools.
///
/// # Returns
///
/// * `ToolSet` - A ToolSet containing all the available tools
pub fn get_all_tools() -> ToolSet {
    ToolSet::builder()
        // Project tools
        .static_tool(project::RequestPermission)
        .static_tool(project::Think)
        
        // File tools
        .static_tool(file::DirectoryTree)
        .static_tool(file::ShowFile)
        .static_tool(file::SearchInFile)
        .static_tool(file::EditFile)
        .static_tool(file::WriteFile)
        .static_tool(file::ExecuteShellCommand)
        
        // Code tools
        .static_tool(code::CodeRepoOverview)
        .static_tool(code::Search)
        .build()
}
