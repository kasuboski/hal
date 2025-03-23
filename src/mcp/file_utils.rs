use std::path::{Path, PathBuf};
use std::io::Error as IoError;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use regex::Regex;
use tracing::{info, warn};

use super::permissions::{PermissionsRef, basic_path_validation};

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
    is_regex: bool
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
        let regex = Regex::new(pattern)
            .map_err(|e| format!("Invalid regex pattern: {}", e))?;
        
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
    new_string: &str
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
    append: bool
) -> Result<(), String> {
    // Validate path
    basic_path_validation(path)?;
    
    // Check permissions (parent directory needs write permission)
    let parent_dir = path.parent()
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
        return Err(format!("Directory does not exist: {}", parent_dir.display()));
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
