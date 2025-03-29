// tools/permissions.rs
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info; // If you want logging similar to mcp

// Re-import the type alias from shared.rs for clarity
use crate::tools::shared::PermissionsRef;

#[derive(Debug, Clone, Default)] // Add Default
pub struct SessionPermissions {
    read_allowed_dirs: HashSet<PathBuf>,
    write_allowed_dirs: HashSet<PathBuf>,
    allowed_commands: HashSet<String>,
}

impl SessionPermissions {
    pub fn new() -> Self {
        // Initialize with default allowed commands if desired
        let mut allowed_commands = HashSet::new();
        allowed_commands.insert("ls".to_string());
        // ... add other safe defaults ...
        Self {
            read_allowed_dirs: HashSet::new(),
            write_allowed_dirs: HashSet::new(),
            allowed_commands,
        }
    }

    pub fn can_read(&self, path: &Path) -> bool {
        self.has_permission(path, &self.read_allowed_dirs)
    }

    pub fn can_write(&self, path: &Path) -> bool {
        // Check the parent directory for file operations
        let check_path = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };
        self.has_permission(check_path, &self.write_allowed_dirs)
    }

    // Helper for permission checking (handles canonicalization)
    fn has_permission(&self, path: &Path, allowed_dirs: &HashSet<PathBuf>) -> bool {
        // Implement canonicalization and starts_with check
        // Handle potential canonicalization errors gracefully
        let canonical_path = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // If path doesn't exist yet (e.g., writing a new file),
                // check its parent.
                if let Some(parent) = path.parent() {
                    match parent.canonicalize() {
                        Ok(parent_canon) => {
                            return allowed_dirs
                                .iter()
                                .any(|allowed| parent_canon.starts_with(allowed));
                        }
                        Err(_) => return false, // Cannot check parent
                    }
                } else {
                    return false; // Cannot determine parent
                }
            }
        };
        allowed_dirs
            .iter()
            .any(|allowed| canonical_path.starts_with(allowed))
    }

    pub fn can_execute_command(&self, command: &str) -> bool {
        // Implement logic: split command, check first word against allowed_commands
        let program = command.split_whitespace().next().unwrap_or("");
        self.allowed_commands.contains(program)
    }

    pub fn allow_read(&mut self, dir: PathBuf) {
        info!("Granting read permission for directory: {}", dir.display());
        // Implement logic: canonicalize, insert into read_allowed_dirs
        if let Ok(canonical_path) = dir.canonicalize() {
            self.read_allowed_dirs.insert(canonical_path);
        } else {
            self.read_allowed_dirs.insert(dir); // Store as-is if canonicalization fails
        }
    }

    pub fn allow_write(&mut self, dir: PathBuf) {
        info!("Granting write permission for directory: {}", dir.display());
        // Implement logic: canonicalize, insert into write_allowed_dirs AND read_allowed_dirs
        if let Ok(canonical_path) = dir.canonicalize() {
            self.write_allowed_dirs.insert(canonical_path.clone());
            self.read_allowed_dirs.insert(canonical_path); // Write implies read
        } else {
            self.write_allowed_dirs.insert(dir.clone());
            self.read_allowed_dirs.insert(dir);
        }
    }

    pub fn allow_command(&mut self, command: String) {
        info!("Adding command to allowlist: {}", command);
        // Ensure only the command name (first word) is added if necessary,
        // or allow the full string based on can_execute_command implementation.
        // Assuming can_execute_command checks the first word:
        let program = command
            .split_whitespace()
            .next()
            .unwrap_or(&command)
            .to_string();
        self.allowed_commands.insert(program);
    }
}

// Function to create the shared permissions object
pub fn create_permissions() -> PermissionsRef {
    Arc::new(Mutex::new(SessionPermissions::new()))
}

// Basic path validation function
pub fn basic_path_validation(path: &Path) -> Result<(), String> {
    // Define dangerous_paths list
    let dangerous_paths: &[&'static str] = &["/etc", "/bin", "/dev", "/usr", "/tmp"];
    // Implement logic: canonicalize path_to_check, loop through dangerous_paths, check starts_with
    let path_to_check = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    for dangerous in dangerous_paths.iter() {
        if path_to_check.starts_with(dangerous) {
            return Err(format!("Access to system directory denied: {}", dangerous));
        }
    }
    Ok(())
}
