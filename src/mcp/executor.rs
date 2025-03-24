use std::path::Path;

#[async_trait::async_trait]
pub trait Executor {
    async fn execute(
        &self,
        command: String,
        working_dir: Option<&Path>,
    ) -> Result<CommandResult, Box<dyn std::error::Error>>;
}

/// Result structure for shell command execution
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}
