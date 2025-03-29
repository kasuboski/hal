//! Tool implementations for Hal using the Rig library
//!
//! This module contains implementations of various tools that can be used
//! with LLM agents. The tools are organized into categories:
//!
//! - Project tools: For project setup, permissions, and general operations
//! - File tools: For file system operations like reading, writing, and searching
//! - Code tools: For code analysis and search operations

// Declare modules
pub mod error;
pub mod executor;
pub mod permissions;
pub mod shared; // Contains the State struct

pub mod code;
pub mod file;
pub mod project;

// Import necessary items
use crate::tools::shared::State; // Import the shared State
use rig::tool::{ToolDyn, ToolEmbedding as _, ToolSet};

/// Get all tools packaged in a ToolSet
///
/// Creates and returns a ToolSet with all the tools implemented by HAL.
/// This initializes tools requiring shared state using the provided `state`.
///
/// # Parameters
///
/// * `state` - A shared state instance containing permissions and executor
///
/// # Returns
///
/// * `ToolSet` - A ToolSet containing all the available tools, properly initialized
pub fn get_full_toolset(state: &State) -> ToolSet {
    // Use expect as these initializations should not fail if State is valid
    ToolSet::builder()
        // Project tools - Initialized via init(State, Context)
        .static_tool(
            project::Init::init(state.clone(), ())
                .expect("Failed to initialize project::Init tool"),
        )
        .static_tool(
            project::RequestPermission::init(state.clone(), ())
                .expect("Failed to initialize project::RequestPermission tool"),
        )
        .static_tool(
            project::Think::init(state.clone(), ())
                .expect("Failed to initialize project::Think tool"), // Takes State even if ignored internally
        )
        // File tools - Initialized via init(State, Context)
        .static_tool(
            file::DirectoryTree::init(state.clone(), ())
                .expect("Failed to initialize file::DirectoryTree tool"),
        )
        .static_tool(
            file::ShowFile::init(state.clone(), ())
                .expect("Failed to initialize file::ShowFile tool"),
        )
        .static_tool(
            file::SearchInFile::init(state.clone(), ())
                .expect("Failed to initialize file::SearchInFile tool"),
        )
        .static_tool(
            file::EditFile::init(state.clone(), ())
                .expect("Failed to initialize file::EditFile tool"),
        )
        .static_tool(
            file::WriteFile::init(state.clone(), ())
                .expect("Failed to initialize file::WriteFile tool"),
        )
        .static_tool(
            file::ExecuteShellCommand::init(state.clone(), ())
                .expect("Failed to initialize file::ExecuteShellCommand tool"),
        )
        // Code tools - Initialized via init((), ()), assuming State=() and Context=()
        // If code tools are updated later to use State, their init calls will need state.clone()
        .static_tool(
            code::CodeRepoOverview::init((), ())
                .expect("Failed to initialize code::CodeRepoOverview tool"),
        )
        .static_tool(code::Search::init((), ()).expect("Failed to initialize code::Search tool"))
        .build()
}

/// Get all tools as a Vec of dynamic trait objects
///
/// Creates and returns a Vec containing all tools, boxed as dynamic trait objects.
/// Initializes tools requiring shared state using the provided `state`.
///
/// # Parameters
///
/// * `state` - A shared state instance containing permissions and executor
///
/// # Returns
///
/// * `Vec<Box<dyn ToolDyn + 'static>>` - A Vec of all available tools
pub fn get_all_tools(state: &State) -> Vec<Box<dyn ToolDyn + 'static>> {
    // Use expect as these initializations should not fail if State is valid
    vec![
        // Project tools
        Box::new(
            project::Init::init(state.clone(), ())
                .expect("Failed to initialize project::Init tool"),
        ),
        Box::new(
            project::RequestPermission::init(state.clone(), ())
                .expect("Failed to initialize project::RequestPermission tool"),
        ),
        Box::new(
            project::Think::init(state.clone(), ())
                .expect("Failed to initialize project::Think tool"),
        ),
        // File tools
        Box::new(
            file::DirectoryTree::init(state.clone(), ())
                .expect("Failed to initialize file::DirectoryTree tool"),
        ),
        Box::new(
            file::ShowFile::init(state.clone(), ())
                .expect("Failed to initialize file::ShowFile tool"),
        ),
        Box::new(
            file::SearchInFile::init(state.clone(), ())
                .expect("Failed to initialize file::SearchInFile tool"),
        ),
        Box::new(
            file::EditFile::init(state.clone(), ())
                .expect("Failed to initialize file::EditFile tool"),
        ),
        Box::new(
            file::WriteFile::init(state.clone(), ())
                .expect("Failed to initialize file::WriteFile tool"),
        ),
        Box::new(
            file::ExecuteShellCommand::init(state.clone(), ())
                .expect("Failed to initialize file::ExecuteShellCommand tool"),
        ),
        // Code tools
        Box::new(
            code::CodeRepoOverview::init((), ())
                .expect("Failed to initialize code::CodeRepoOverview tool"),
        ),
        Box::new(code::Search::init((), ()).expect("Failed to initialize code::Search tool")),
    ]
}
