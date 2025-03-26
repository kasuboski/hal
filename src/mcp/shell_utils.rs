//! Shell command utilities for the MCP server
//!
//! This module provides secure shell command execution with permission checks:
//! - Command execution with stdout/stderr capture through the user's default shell
//! - Command validation to prevent shell injection
//! - Permission checking against an allowlist
//!
//! The implementation focuses on security by:
//! - Checking permissions against an allowlist
//! - Supporting a working directory specification
//! - Providing clear error messages for failures
//! - Detecting and using the user's default shell

use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;

use super::executor::{CommandResult, Executor};
use super::permissions::PermissionsRef;

/// ShellExecutor implements the Executor trait
/// It reuses the shell between command executions
pub struct ShellExecutor {
    permissions: PermissionsRef,
    shell_path: Arc<Mutex<Option<String>>>,
}

impl ShellExecutor {
    /// Create a new ShellExecutor
    pub fn new(permissions: PermissionsRef) -> Self {
        ShellExecutor {
            permissions,
            shell_path: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize the shell path if not already done
    async fn ensure_shell_initialized(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut shell_path_guard = self.shell_path.lock().await;

        if shell_path_guard.is_none() {
            // Initialize the shell path if it hasn't been done yet
            let detected_shell = detect_default_shell().await?;
            *shell_path_guard = Some(detected_shell);
        }

        Ok(shell_path_guard.as_ref().unwrap().clone())
    }
}

#[async_trait::async_trait]
impl Executor for ShellExecutor {
    async fn execute(
        &self,
        command: String,
        working_dir: Option<&Path>,
    ) -> Result<CommandResult, Box<dyn std::error::Error>> {
        let command_str = command;

        // Check if command is allowed
        let perms = self.permissions.lock().await;
        if !perms.can_execute_command(&command_str) {
            return Err(format!(
                "Command not in allowlist: {}. Only safe, read-only commands are permitted.",
                command_str
            )
            .into());
        }

        // Ensure we have initialized the shell
        let shell = self.ensure_shell_initialized().await?;

        // Create command using the detected shell
        let mut command = TokioCommand::new(&shell);
        command.args(&["-c", &command_str]);

        // Set working directory if specified
        if let Some(dir) = working_dir {
            // Verify read permission for working directory
            if !perms.can_read(dir) {
                return Err(format!(
                    "Read permission not granted for directory: {}. Request permission first.",
                    dir.display()
                )
                .into());
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
}

/// Detect the user's default shell based on the operating system
async fn detect_default_shell() -> Result<String, Box<dyn std::error::Error>> {
    // Platform-specific detection methods
    #[cfg(target_os = "macos")]
    {
        if let Some(shell) = get_macos_shell().await {
            return Ok(shell);
        }
        // macOS fallback to zsh
        return Ok("/bin/zsh".to_string());
    }

    #[cfg(all(target_family = "unix", not(target_os = "macos")))]
    {
        if let Some(shell) = get_unix_shell().await {
            return Ok(shell);
        }
        // Linux/Unix fallback to sh
        return Ok("/bin/sh".to_string());
    }

    #[allow(unreachable_code)]
    // General fallback
    Err(Box::new(io::Error::new(
        io::ErrorKind::NotFound,
        "Could not detect default shell",
    )))
}

/// Get default shell on macOS using dscl
#[cfg(target_os = "macos")]
async fn get_macos_shell() -> Option<String> {
    // Try using dscl command
    let username = std::env::var("USER").ok()?;
    let user_path = format!("/Users/{}", username);

    let output = TokioCommand::new("dscl")
        .args(&[".", "-read", &user_path, "UserShell"])
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let output_str = String::from_utf8(output.stdout).ok()?;
        let shell = output_str
            .lines()
            .find(|line| line.contains("UserShell:"))
            .map(|line| line.trim_start_matches("UserShell: ").trim().to_string())?;

        // Verify shell exists
        if tokio::fs::metadata(&shell).await.is_ok() {
            return Some(shell);
        }
    }

    None
}

/// Get default shell on Unix systems from passwd file
#[cfg(target_family = "unix")]
#[allow(dead_code)]
async fn get_unix_shell() -> Option<String> {
    // Get current username using tokio
    let output = TokioCommand::new("whoami").output().await.ok()?;

    let username = String::from_utf8(output.stdout)
        .ok()
        .map(|s| s.trim().to_owned())?;

    // Read /etc/passwd file asynchronously
    let file = File::open("/etc/passwd").await.ok()?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await.ok()? {
        let fields: Vec<&str> = line.split(':').collect();

        if fields.len() >= 7 && fields[0] == username {
            let shell = fields[6].to_string();

            // Verify shell exists asynchronously
            if tokio::fs::metadata(&shell).await.is_ok() {
                return Some(shell);
            }
        }
    }

    None
}
