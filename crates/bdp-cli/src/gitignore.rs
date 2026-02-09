//! .gitignore management for BDP projects
//!
//! Automatically manages .gitignore entries for BDP project files.
//! The entire `.bdp/` directory is gitignored since it contains
//! local cache, config, and database files.

use std::{fs, path::Path};

use crate::error::Result;

/// Marker comment for BDP section in .gitignore
const BDP_SECTION_MARKER: &str = "# BDP cache and runtime files";

/// Entries to add to .gitignore for BDP
const BDP_ENTRIES: &[&str] = &[".bdp/"];

/// Update .gitignore with BDP entries
///
/// This function is idempotent - it can be called multiple times safely.
/// - If .gitignore doesn't exist, creates it with BDP entries
/// - If .gitignore exists but doesn't have BDP section, appends it
/// - If old-style BDP section exists with individual entries, replaces it
/// - If BDP section exists with current entries, does nothing
pub fn update_gitignore(project_dir: &Path) -> Result<()> {
    let gitignore_path = project_dir.join(".gitignore");

    if !gitignore_path.exists() {
        // Create new .gitignore with BDP section
        create_gitignore(&gitignore_path)?;
    } else {
        // Update existing .gitignore
        append_to_gitignore(&gitignore_path)?;
    }

    Ok(())
}

/// Create a new .gitignore file with BDP entries
fn create_gitignore(path: &Path) -> Result<()> {
    let content = format_bdp_section();
    fs::write(path, content)?;
    Ok(())
}

/// Append BDP section to existing .gitignore
fn append_to_gitignore(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path)?;

    // Check if BDP section already exists
    if content.contains(BDP_SECTION_MARKER) {
        // Section exists - check if it needs updating (migration from old entries)
        if has_all_entries(&content) {
            return Ok(()); // Nothing to do
        }
        // Replace the existing section with new simplified entries
        replace_bdp_section(path, &content)?;
    } else {
        // Append new section
        let mut new_content = content;
        if !new_content.is_empty() && !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        if !new_content.is_empty() {
            new_content.push('\n');
        }
        new_content.push_str(&format_bdp_section());
        fs::write(path, new_content)?;
    }

    Ok(())
}

/// Format the BDP section for .gitignore
fn format_bdp_section() -> String {
    let mut section = String::new();
    section.push_str(BDP_SECTION_MARKER);
    section.push('\n');
    for entry in BDP_ENTRIES {
        section.push_str(entry);
        section.push('\n');
    }
    section
}

/// Check if all BDP entries are present as exact lines in the content
fn has_all_entries(content: &str) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    BDP_ENTRIES
        .iter()
        .all(|entry| lines.iter().any(|line| line.trim() == *entry))
}

/// Replace the existing BDP section with updated entries.
/// Handles migration from old-style individual entries to single `.bdp/`.
fn replace_bdp_section(path: &Path, content: &str) -> Result<()> {
    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if line == BDP_SECTION_MARKER {
            // Replace entire BDP section with new format
            new_lines.push(BDP_SECTION_MARKER.to_string());
            for entry in BDP_ENTRIES {
                new_lines.push((*entry).to_string());
            }
            i += 1;

            // Skip old section lines
            while i < lines.len() {
                let old_line = lines[i];
                if old_line.trim().is_empty() {
                    break;
                }
                if old_line.starts_with('#') && old_line != BDP_SECTION_MARKER {
                    break;
                }
                i += 1; // Skip old entry
            }
        } else {
            new_lines.push(line.to_string());
            i += 1;
        }
    }

    let mut result = new_lines.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    fs::write(path, result)?;
    Ok(())
}

/// Remove BDP entries from .gitignore
///
/// Useful for cleanup or testing
#[allow(dead_code)]
pub fn remove_from_gitignore(project_dir: &Path) -> Result<()> {
    let gitignore_path = project_dir.join(".gitignore");

    if !gitignore_path.exists() {
        return Ok(()); // Nothing to do
    }

    let content = fs::read_to_string(&gitignore_path)?;

    if !content.contains(BDP_SECTION_MARKER) {
        return Ok(()); // No BDP section
    }

    // Remove BDP section
    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines = Vec::new();
    let mut in_bdp_section = false;

    for line in lines {
        if line == BDP_SECTION_MARKER {
            in_bdp_section = true;
            continue; // Skip marker line
        }

        if in_bdp_section {
            if line.trim().is_empty() {
                in_bdp_section = false;
                // Keep the empty line if not at end
                if !new_lines.is_empty() {
                    new_lines.push(line.to_string());
                }
                continue;
            }
            if line.starts_with('#') {
                // New section starts
                in_bdp_section = false;
                new_lines.push(line.to_string());
                continue;
            }
            // Skip BDP entry lines
            continue;
        }

        new_lines.push(line.to_string());
    }

    // Write back
    let mut result = new_lines.join("\n");
    if content.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    fs::write(gitignore_path, result)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_gitignore_new_file() {
        let temp = TempDir::new().unwrap();
        update_gitignore(temp.path()).unwrap();

        let content = fs::read_to_string(temp.path().join(".gitignore")).unwrap();
        assert!(content.contains(BDP_SECTION_MARKER));
        assert!(content.contains(".bdp/"));
    }

    #[test]
    fn test_gitignore_append() {
        let temp = TempDir::new().unwrap();
        let gitignore = temp.path().join(".gitignore");

        fs::write(&gitignore, "node_modules/\n*.log\n").unwrap();

        update_gitignore(temp.path()).unwrap();

        let content = fs::read_to_string(&gitignore).unwrap();
        assert!(content.contains("node_modules/"));
        assert!(content.contains("*.log"));
        assert!(content.contains(BDP_SECTION_MARKER));
        assert!(content.contains(".bdp/"));
    }

    #[test]
    fn test_gitignore_idempotent() {
        let temp = TempDir::new().unwrap();
        let gitignore = temp.path().join(".gitignore");

        // First update
        update_gitignore(temp.path()).unwrap();
        let content1 = fs::read_to_string(&gitignore).unwrap();

        // Second update
        update_gitignore(temp.path()).unwrap();
        let content2 = fs::read_to_string(&gitignore).unwrap();

        // Should be identical
        assert_eq!(content1, content2);

        // Count occurrences of BDP entry
        let count = content1.matches(".bdp/").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_gitignore_migrates_old_entries() {
        let temp = TempDir::new().unwrap();
        let gitignore = temp.path().join(".gitignore");

        // Create .gitignore with old-style BDP section
        let old_content = format!(
            "{}\n.bdp/cache/\n.bdp/bdp.db\n.bdp/bdp.db-shm\n.bdp/bdp.db-wal\n",
            BDP_SECTION_MARKER
        );
        fs::write(&gitignore, old_content).unwrap();

        // Update should replace with new simplified entries
        update_gitignore(temp.path()).unwrap();

        let content = fs::read_to_string(&gitignore).unwrap();
        // Should contain the single .bdp/ entry
        assert!(content.contains(".bdp/"));
        // Old individual entries should be gone - check line by line
        let lines: Vec<&str> = content.lines().collect();
        assert!(
            !lines.iter().any(|l| *l == ".bdp/cache/"),
            "Should not have .bdp/cache/ as a separate line"
        );
        assert!(
            !lines.iter().any(|l| *l == ".bdp/bdp.db"),
            "Should not have .bdp/bdp.db as a separate line"
        );
        assert!(
            !lines.iter().any(|l| *l == ".bdp/bdp.db-shm"),
            "Should not have .bdp/bdp.db-shm as a separate line"
        );
    }

    #[test]
    fn test_gitignore_preserves_other_content() {
        let temp = TempDir::new().unwrap();
        let gitignore = temp.path().join(".gitignore");

        let initial_content = "# Python\n__pycache__/\n*.pyc\n\n# Node\nnode_modules/\n";
        fs::write(&gitignore, initial_content).unwrap();

        update_gitignore(temp.path()).unwrap();

        let content = fs::read_to_string(&gitignore).unwrap();
        assert!(content.contains("__pycache__/"));
        assert!(content.contains("*.pyc"));
        assert!(content.contains("node_modules/"));
        assert!(content.contains(BDP_SECTION_MARKER));
    }

    #[test]
    fn test_remove_from_gitignore() {
        let temp = TempDir::new().unwrap();

        // Add BDP entries
        update_gitignore(temp.path()).unwrap();

        let gitignore = temp.path().join(".gitignore");
        let content_before = fs::read_to_string(&gitignore).unwrap();
        assert!(content_before.contains(BDP_SECTION_MARKER));

        // Remove BDP entries
        remove_from_gitignore(temp.path()).unwrap();

        let content_after = fs::read_to_string(&gitignore).unwrap();
        assert!(!content_after.contains(BDP_SECTION_MARKER));
        assert!(!content_after.contains(".bdp/"));
    }

    #[test]
    fn test_format_bdp_section() {
        let section = format_bdp_section();
        assert!(section.starts_with(BDP_SECTION_MARKER));
        assert!(section.ends_with('\n'));
        assert!(section.contains(".bdp/"));
    }
}
