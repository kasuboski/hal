use std::future::Future;
// tools/shared.rs
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::tools::error::ToolError;

// Forward declare or define the permissions struct we'll create later
// For now, a simple definition or use a placeholder if SessionPermissions is in another file.
// Let's assume SessionPermissions will be in tools/permissions.rs
use crate::tools::permissions::SessionPermissions;

// Type alias for shared, mutable permissions
pub type PermissionsRef = Arc<Mutex<SessionPermissions>>;

// Define the result structure for command execution
#[derive(Debug)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub trait Executor {
    fn execute(
        &self,
        command: String,
        working_dir: Option<&Path>,
    ) -> Pin<Box<dyn Future<Output = Result<CommandResult, ToolError>> + Send + Sync + '_>>;
}

// The central state holder
#[derive(Clone)]
pub struct State {
    pub permissions: PermissionsRef,
    pub executor: Arc<dyn Executor + Send + Sync>,
    // Add other shared state if needed later
    // pub project_path: Arc<Mutex<Option<String>>>,
}

impl State {
    // Initialize the state. Implementations for permissions and executor
    // will come from subsequent steps.
    pub fn new() -> Self {
        // Create instances of our permission and executor implementations (defined later)
        let permissions = crate::tools::permissions::create_permissions(); // Function to be created in Step 2
        let executor = Arc::new(crate::tools::executor::ShellExecutor::new(
            permissions.clone(),
        )); // Struct to be created in Step 3

        State {
            permissions,
            executor,
        }
    }
}

// Add Default trait if useful
impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
