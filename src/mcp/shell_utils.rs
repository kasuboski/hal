use std::path::Path;
use std::process::Command;
use tokio::process::Command as TokioCommand;
use tracing::{info, warn};

use super::permissions::PermissionsRef;

/// Result structure for shell command execution
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Execute a shell command with permission validation
pub async fn execute_command(
    command_str: &str,
    permissions: &PermissionsRef,
    working_dir: Option<&Path>,
) -> Result<CommandResult, String> {
    // Check if command is allowed
    let perms = permissions.lock().await;
    if !perms.can_execute_command(command_str) {
        return Err(format!(
            "Command not in allowlist: {}. Only safe, read-only commands are permitted.",
            command_str
        ));
    }
    
    // Parse command and arguments
    let mut parts = command_str.split_whitespace();
    let program = parts.next().ok_or_else(|| "Empty command".to_string())?;
    let args: Vec<&str> = parts.collect();
    
    // Create command
    let mut command = TokioCommand::new(program);
    command.args(&args);
    
    // Set working directory if specified
    if let Some(dir) = working_dir {
        // Verify read permission for working directory
        if !perms.can_read(dir) {
            return Err(format!(
                "Read permission not granted for directory: {}. Request permission first.",
                dir.display()
            ));
        }
        command.current_dir(dir);
    }
    
    // Execute command
    let output = command
        .output()
        .await
        .map_err(|e| format!("Failed to execute command: {}", e))?;
    
    // Parse output
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);
    
    Ok(CommandResult {
        stdout,
        stderr,
        exit_code,
    })
}

/// Validate that a command is safe to execute (no pipes, redirects, etc.)
pub fn validate_command(command: &str) -> Result<(), String> {
    // Check for shell metacharacters
    let dangerous_chars = [';', '&', '|', '>', '<', '`', '$', '(', ')', '{', '}', '[', ']', '\\', '\'', '\"'];
    
    for c in dangerous_chars.iter() {
        if command.contains(*c) {
            return Err(format!(
                "Command contains dangerous character '{}'. Only simple commands are allowed.",
                c
            ));
        }
    }
    
    Ok(())
}
