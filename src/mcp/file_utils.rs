//! File operation utilities for the MCP server
//!
//! This module implements file operations with permission checks:
//! - Reading file contents with optional line ranges
//! - Searching files for patterns or regular expressions
//! - Making precise string replacements in files
//! - Writing or appending content to files
//! - Retrieving a directory tree structure
//!
//! Each operation checks permissions and validates paths before proceeding,
//! ensuring security and proper error handling.

use regex::Regex;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use super::permissions::{basic_path_validation, PermissionsRef};

/// Show file contents with optional line range
pub async fn show_file(
    path: &Path,
    permissions: &PermissionsRef,
    start_line: Option<usize>,
    end_line: Option<usize>,
) -> Result<String, String> {
    // Validate path
    basic_path_validation(path)?;

    // Check permissions
    let perms = permissions.lock().await;
    if !perms.can_read(path) {
        return Err(format!(
            "Read permission not granted for path: {}. Request permission first.",
            path.display()
        ));
    }

    // Read file
    let content = fs::read_to_string(path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Apply line range if specified
    if start_line.is_some() || end_line.is_some() {
        let lines: Vec<&str> = content.lines().collect();
        let start = start_line.unwrap_or(1).saturating_sub(1);
        let end = end_line.unwrap_or(lines.len()).min(lines.len());

        if start >= end || start >= lines.len() {
            return Err(format!("Invalid line range: {} to {}", start + 1, end));
        }

        Ok(lines[start..end].join("\n"))
    } else {
        Ok(content)
    }
}

/// Search for a pattern in a file
pub async fn search_in_file(
    path: &Path,
    permissions: &PermissionsRef,
    pattern: &str,
    is_regex: bool,
) -> Result<Vec<(usize, String)>, String> {
    // Validate path
    basic_path_validation(path)?;

    // Check permissions
    let perms = permissions.lock().await;
    if !perms.can_read(path) {
        return Err(format!(
            "Read permission not granted for path: {}. Request permission first.",
            path.display()
        ));
    }

    // Read file
    let content = fs::read_to_string(path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let lines: Vec<&str> = content.lines().collect();
    let mut matches = Vec::new();

    if is_regex {
        // Compile regex pattern
        let regex = Regex::new(pattern).map_err(|e| format!("Invalid regex pattern: {}", e))?;

        // Search for matches
        for (i, line) in lines.iter().enumerate() {
            if regex.is_match(line) {
                matches.push((i + 1, line.to_string()));
            }
        }
    } else {
        // Simple string search
        for (i, line) in lines.iter().enumerate() {
            if line.contains(pattern) {
                matches.push((i + 1, line.to_string()));
            }
        }
    }

    Ok(matches)
}

/// Edit a file by replacing a string
pub async fn edit_file(
    path: &Path,
    permissions: &PermissionsRef,
    old_string: &str,
    new_string: &str,
) -> Result<(), String> {
    // Validate path
    basic_path_validation(path)?;

    // Check permissions
    let perms = permissions.lock().await;
    if !perms.can_write(path) {
        return Err(format!(
            "Write permission not granted for path: {}. Request permission first.",
            path.display()
        ));
    }

    // Read file
    let content = fs::read_to_string(path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Count occurrences of old_string
    let occurrences = content.matches(old_string).count();
    if occurrences == 0 {
        return Err(format!("String not found in file: {}", path.display()));
    } else if occurrences > 1 {
        return Err(format!("Found {} occurrences of the string in file. Please provide more context to make the match unique.", occurrences));
    }

    // Replace string and write back to file
    let new_content = content.replace(old_string, new_string);
    fs::write(path, new_content)
        .await
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

/// Write content to a file (create or overwrite)
pub async fn write_file(
    path: &Path,
    permissions: &PermissionsRef,
    content: &str,
    append: bool,
) -> Result<(), String> {
    // Validate path
    basic_path_validation(path)?;

    // Check permissions (parent directory needs write permission)
    let parent_dir = path
        .parent()
        .ok_or_else(|| "Invalid path: no parent directory".to_string())?;

    let perms = permissions.lock().await;
    if !perms.can_write(parent_dir) {
        return Err(format!(
            "Write permission not granted for directory: {}. Request permission first.",
            parent_dir.display()
        ));
    }

    // Make sure parent directory exists
    if !parent_dir.exists() {
        return Err(format!(
            "Directory does not exist: {}",
            parent_dir.display()
        ));
    }

    // Write or append to file
    if append {
        // Create file if it doesn't exist, or append to it
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .map_err(|e| format!("Failed to open file for appending: {}", e))?;

        file.write_all(content.as_bytes())
            .await
            .map_err(|e| format!("Failed to append to file: {}", e))?;
    } else {
        // Create or overwrite file
        fs::write(path, content)
            .await
            .map_err(|e| format!("Failed to write file: {}", e))?;
    }

    Ok(())
}

/// Get a directory tree from a path
///
/// Lists directories and files in the specified directory with their hierarchical structure.
/// Requires read permission for the directory.
///
/// # Arguments
///
/// * `path` - Path to the directory
/// * `permissions` - Reference to permissions object
///
/// # Returns
///
/// * `Result<Vec<String>, String>` - List of paths in tree format or error message
pub async fn directory_tree(
    path: &Path,
    permissions: &PermissionsRef,
) -> Result<Vec<String>, String> {
    // Validate path
    basic_path_validation(path)?;

    // Check permissions
    let perms = permissions.lock().await;
    if !perms.can_read(path) {
        return Err(format!(
            "Read permission not granted for path: {}. Request permission first.",
            path.display()
        ));
    }

    // Verify directory exists and is a directory
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", path.display()));
    }

    // Build the tree structure
    let mut result = Vec::new();
    let root_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    result.push(root_name);
    build_tree_structure(path, &mut result, String::from("  "), 1).await?;

    Ok(result)
}

/// Helper function to recursively build the directory tree structure
///
/// # Arguments
///
/// * `dir_path` - Current directory path
/// * `result` - Vector to store tree entries
/// * `prefix` - String prefix for the current level
/// * `max_depth` - Maximum recursion depth (to prevent excessive output)
///
/// # Returns
///
/// * `Result<(), String>` - Ok on success or error message
async fn build_tree_structure(
    dir_path: &Path,
    result: &mut Vec<String>,
    prefix: String,
    depth: usize,
) -> Result<(), String> {
    // Guard against too deep recursion
    if depth > 10 {
        result.push(format!("{}... (max depth reached)", prefix));
        return Ok(());
    }

    // Read directory entries
    let mut entries = match fs::read_dir(dir_path).await {
        Ok(entries) => entries,
        Err(e) => return Err(format!("Failed to read directory: {}", e)),
    };

    // Process all entries
    let mut entry_list = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Skip hidden files and directories (starting with .)
        if name.starts_with('.') {
            continue;
        }

        entry_list.push((path, name));
    }

    // Sort entries: directories first, then files, both alphabetically
    entry_list.sort_by(|(path_a, name_a), (path_b, name_b)| {
        let is_dir_a = path_a.is_dir();
        let is_dir_b = path_b.is_dir();

        if is_dir_a && !is_dir_b {
            std::cmp::Ordering::Less
        } else if !is_dir_a && is_dir_b {
            std::cmp::Ordering::Greater
        } else {
            name_a.cmp(name_b)
        }
    });

    // Process each entry
    for (i, (path, name)) in entry_list.iter().enumerate() {
        let is_last = i == entry_list.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };

        let entry_prefix = format!("{}{}", prefix, connector);
        result.push(format!("{}{}", entry_prefix, name));

        // Recursively process subdirectories
        if path.is_dir() {
            let next_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };

            // Use Box::pin to handle the recursive async call
            let build_future = Box::pin(build_tree_structure(path, result, next_prefix, depth + 1));
            build_future.await?;
        }
    }

    Ok(())
}
