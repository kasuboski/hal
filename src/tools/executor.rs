// tools/executor.rs

use crate::tools::shared::{CommandResult, Executor, PermissionsRef};
use anyhow::Result;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;
use tracing::info;

pub struct ShellExecutor {
    permissions: PermissionsRef,
    shell_path: Arc<Mutex<Option<String>>>,
}

impl ShellExecutor {
    pub fn new(permissions: PermissionsRef) -> Self {
        Self {
            permissions,
            shell_path: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize the shell path if not already done
    async fn ensure_shell_initialized(&self) -> Result<String, ToolError> {
        let mut shell_path_guard = self.shell_path.lock().await;

        if shell_path_guard.is_none() {
            // Initialize the shell path if it hasn't been done yet
            let detected_shell = detect_default_shell().await?;
            *shell_path_guard = Some(detected_shell);
        }

        Ok(shell_path_guard.as_ref().unwrap().clone())
    }
}

impl Executor for ShellExecutor {
    fn execute(
        &self,
        command_str: String,
        working_dir: Option<&Path>,
    ) -> Pin<Box<dyn Future<Output = Result<CommandResult, ToolError>> + Send + Sync + '_>> {
        // If working_dir is Some, clone it into an owned PathBuf
        // This is often safer for async blocks, avoiding lifetime issues.
        let working_dir_owned: Option<PathBuf> = working_dir.map(|p| p.to_path_buf());

        Box::pin(async move {
            // --- Start of original async fn body ---

            // 1. Check execute permission
            {
                // `self` is captured by the async move block.
                // This requires ShellExecutor: Sync for the future to be Sync.
                let perms = self.permissions.lock().await;
                if !perms.can_execute_command(&command_str) {
                    return Err(ToolError::PermissionDenied(format!(
                        "Command execution denied: '{}'",
                        command_str
                    )));
                }
            }

            // 2. Check working directory read permission if specified
            if let Some(ref dir) = working_dir_owned {
                // Use the owned path
                {
                    let perms = self.permissions.lock().await;
                    if !perms.can_read(dir) {
                        return Err(ToolError::PermissionDenied(format!(
                            "Read permission denied for working directory: '{}'",
                            dir.display() // Use the owned path
                        )));
                    }
                }
            }

            // 3. Get shell
            // ensure_shell_initialized must also be callable on &self
            // and the future it returns should be Send.
            let shell = self.ensure_shell_initialized().await?;

            // 4. Prepare command
            info!("running command {} with shell {}", command_str, shell);
            let mut command = TokioCommand::new(shell); // Use a different variable name

            if cfg!(target_os = "windows") {
                command.args(["/C", &command_str]);
            } else {
                command.args(["-c", &command_str]);
            };

            if let Some(ref dir) = working_dir_owned {
                // Use the owned path
                command.current_dir(dir); // Use the owned path
            }

            // 5. Execute and capture output
            // TokioCommand::output returns a future that is Send.
            let output = command.output().await?;

            // 6. Parse and return result
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);

            Ok(CommandResult {
                stdout,
                stderr,
                exit_code,
            })
            // --- End of original async fn body ---
        })
    }
}

/// Detect the user's default shell based on the operating system
use crate::tools::error::ToolError;
async fn detect_default_shell() -> Result<String, ToolError> {
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
    Err(ToolError::ShellDetectionError(
        "Could not detect default shell".to_string(),
    ))
}

/// Get default shell on macOS using dscl
#[cfg(target_os = "macos")]
async fn get_macos_shell() -> Option<String> {
    // Try using dscl command
    let username = std::env::var("USER").ok()?;
    let user_path = format!("/Users/{}", username);

    let output = TokioCommand::new("dscl")
        .args([".", "-read", &user_path, "UserShell"])
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

    use tokio::{
        fs::File,
        io::{AsyncBufReadExt as _, BufReader},
    };
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
