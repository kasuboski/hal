use std::path::Path;

// Remove async_trait and make it a normal trait with an async method
pub trait Executor: Send + Sync {
    fn execute<'a>(
        &'a self,
        command: String,
        working_dir: Option<&'a Path>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<CommandResult, Box<dyn std::error::Error>>>
                + Send
                + 'a,
        >,
    >;
}

/// Result structure for shell command execution
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}
