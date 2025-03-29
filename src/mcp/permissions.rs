//! Permission management for the MCP server
//!
//! This module implements the session-based permission system that tracks which
//! directories have read/write permissions and which shell commands are allowed.
//! It provides:
//!
//! - A thread-safe permission structure that persists throughout the session
//! - Functions to check if operations are allowed
//! - Methods to grant new permissions
//! - Path validation to prevent access to sensitive system directories

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// Session permissions structure to track allowed directories and commands
///
/// This structure maintains three sets of permissions:
/// - Directories with read permission
/// - Directories with write permission
/// - Allowed shell commands (simple allowlist)
///
/// The permission state is maintained throughout the session, so permissions
/// only need to be granted once for each directory or command.
#[derive(Debug, Clone)]
pub struct SessionPermissions {
    /// Directories with read permission
    read_allowed_dirs: HashSet<PathBuf>,

    /// Directories with write permission
    write_allowed_dirs: HashSet<PathBuf>,

    /// Allowed shell commands (simple allowlist)
    allowed_commands: HashSet<String>,
}

impl Default for SessionPermissions {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionPermissions {
    pub fn new() -> Self {
        // Default allowed shell commands
        let mut allowed_commands = HashSet::new();
        allowed_commands.insert("ls".to_string());
        allowed_commands.insert("cat".to_string());
        allowed_commands.insert("grep".to_string());
        allowed_commands.insert("find".to_string());
        allowed_commands.insert("echo".to_string());
        allowed_commands.insert("pwd".to_string());
        allowed_commands.insert("wc".to_string());
        allowed_commands.insert("head".to_string());
        allowed_commands.insert("tail".to_string());
        allowed_commands.insert("which".to_string());

        Self {
            read_allowed_dirs: HashSet::new(),
            write_allowed_dirs: HashSet::new(),
            allowed_commands,
        }
    }

    /// Check if read is allowed for a path
    pub fn can_read(&self, path: &Path) -> bool {
        self.has_permission(path, &self.read_allowed_dirs)
    }

    /// Check if write is allowed for a path
    pub fn can_write(&self, path: &Path) -> bool {
        self.has_permission(path, &self.write_allowed_dirs)
    }

    /// Check if a path is within any of the allowed directories
    fn has_permission(&self, path: &Path, allowed_dirs: &HashSet<PathBuf>) -> bool {
        // First try to canonicalize the path
        let canonical_path = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // If we can't canonicalize, check if the parent directory is allowed
                // This is useful for operations like writing to a new file
                if let Some(parent) = path.parent() {
                    match parent.canonicalize() {
                        Ok(parent_path) => {
                            return allowed_dirs.iter().any(|dir| parent_path.starts_with(dir));
                        }
                        Err(_) => return false,
                    }
                }
                return false;
            }
        };

        // Check if path is within any allowed directory
        allowed_dirs
            .iter()
            .any(|dir| canonical_path.starts_with(dir))
    }

    /// Check if a shell command is allowed
    pub fn can_execute_command(&self, command: &str) -> bool {
        // Extract the program name (first word)
        let program = command.split_whitespace().next().unwrap_or("");
        self.allowed_commands.contains(program)
    }

    /// Grant read permission for a directory
    pub fn allow_read(&mut self, dir: PathBuf) {
        info!("Granting read permission for directory: {}", dir.display());
        // Try to canonicalize the path
        if let Ok(canonical_path) = dir.canonicalize() {
            self.read_allowed_dirs.insert(canonical_path);
        } else {
            // If canonicalization fails, use the path as is
            self.read_allowed_dirs.insert(dir);
        }
    }

    /// Grant write permission for a directory
    pub fn allow_write(&mut self, dir: PathBuf) {
        info!("Granting write permission for directory: {}", dir.display());
        // Try to canonicalize the path
        if let Ok(canonical_path) = dir.canonicalize() {
            self.write_allowed_dirs.insert(canonical_path.clone());
            // Write permission implies read permission
            self.read_allowed_dirs.insert(canonical_path);
        } else {
            // If canonicalization fails, use the path as is
            self.write_allowed_dirs.insert(dir.clone());
            self.read_allowed_dirs.insert(dir);
        }
    }

    /// Allow a new shell command
    pub fn allow_command(&mut self, command: String) {
        info!("Adding command to allowlist: {}", command);
        self.allowed_commands.insert(command);
    }
}

/// Type alias for a thread-safe reference to session permissions
pub type PermissionsRef = Arc<Mutex<SessionPermissions>>;

/// Create a new shared permissions object
pub fn create_permissions() -> PermissionsRef {
    Arc::new(Mutex::new(SessionPermissions::new()))
}

/// Basic validation to prevent access to system directories
pub fn basic_path_validation(path: &Path) -> Result<(), String> {
    // Check for obviously dangerous paths
    let dangerous_paths = [
        "/etc",
        "/bin",
        "/sbin",
        "/usr/bin",
        "/usr/sbin",
        "/boot",
        "/lib",
        "/lib64",
        "/dev",
        "/proc",
        "/sys",
        "/var/run",
        "/var/log",
        "/var/lib",
        "/var/tmp",
    ];

    // Convert to absolute path if possible
    let path_to_check = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    for dangerous in dangerous_paths.iter() {
        if path_to_check.starts_with(dangerous) {
            return Err(format!("Cannot access system directory: {}", dangerous));
        }
    }

    Ok(())
}
