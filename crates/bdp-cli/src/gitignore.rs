//! .gitignore management for BDP projects
//!
//! Automatically manages .gitignore entries for BDP cache and runtime files.

use crate::error::Result;
use std::fs;
use std::path::Path;

/// Marker comment for BDP section in .gitignore
const BDP_SECTION_MARKER: &str = "# BDP cache and runtime files";

/// Entries to add to .gitignore for BDP
const BDP_ENTRIES: &[&str] = &[
    ".bdp/cache/",
    ".bdp/bdp.db",
    ".bdp/bdp.db-shm",
    ".bdp/bdp.db-wal",
    ".bdp/resolved-dependencies.json",
    ".bdp/audit.log",
];

/// Update .gitignore with BDP entries
///
/// This function is idempotent - it can be called multiple times safely.
/// - If .gitignore doesn't exist, creates it with BDP entries
/// - If .gitignore exists but doesn't have BDP section, appends it
/// - If BDP section exists, ensures all entries are present
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
        // Section exists - check if all entries are present
        if has_all_entries(&content) {
            return Ok(()); // Nothing to do
        }
        // Update existing section
        update_bdp_section(path, &content)?;
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

/// Check if all BDP entries are present in the content
fn has_all_entries(content: &str) -> bool {
    BDP_ENTRIES.iter().all(|entry| content.contains(entry))
}

/// Update the existing BDP section with missing entries
fn update_bdp_section(path: &Path, content: &str) -> Result<()> {
    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines = Vec::new();
    let mut bdp_section_lines = Vec::new();

    // Find the BDP section
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        if line == BDP_SECTION_MARKER {
            // Found BDP section marker
            new_lines.push(line.to_string());
            bdp_section_lines.clear();
            i += 1;

            // Collect existing BDP section lines
            while i < lines.len() {
                let bdp_line = lines[i];
                if bdp_line.trim().is_empty() {
                    // Empty line marks end of section
                    break;
                }
                if bdp_line.starts_with('#') && bdp_line != BDP_SECTION_MARKER {
                    // New comment section starts
                    break;
                }
                bdp_section_lines.push(bdp_line);
                i += 1;
            }

            // Add all required entries (deduplicated)
            let mut added_entries = std::collections::HashSet::new();
            for existing_line in &bdp_section_lines {
                added_entries.insert(existing_line.trim());
                new_lines.push(existing_line.to_string());
            }

            // Add missing entries
            for entry in BDP_ENTRIES {
                if !added_entries.contains(*entry) {
                    new_lines.push(entry.to_string());
                }
            }
        } else {
            new_lines.push(line.to_string());
            i += 1;
        }
    }

    // Write updated content
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_gitignore_new_file() {
        let temp = TempDir::new().unwrap();
        update_gitignore(temp.path()).unwrap();

        let content = fs::read_to_string(temp.path().join(".gitignore")).unwrap();
        assert!(content.contains(BDP_SECTION_MARKER));
        for entry in BDP_ENTRIES {
            assert!(content.contains(entry), "Missing entry: {}", entry);
        }
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
        assert!(content.contains(".bdp/cache/"));
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

        // Count occurrences of BDP entries
        let count1 = content1.matches(".bdp/cache/").count();
        let count2 = content2.matches(".bdp/cache/").count();
        assert_eq!(count1, 1);
        assert_eq!(count2, 1);
    }

    #[test]
    fn test_gitignore_updates_incomplete_section() {
        let temp = TempDir::new().unwrap();
        let gitignore = temp.path().join(".gitignore");

        // Create .gitignore with incomplete BDP section
        let initial_content = format!(
            "{}\n.bdp/cache/\n.bdp/bdp.db\n",
            BDP_SECTION_MARKER
        );
        fs::write(&gitignore, initial_content).unwrap();

        // Update should add missing entries
        update_gitignore(temp.path()).unwrap();

        let content = fs::read_to_string(&gitignore).unwrap();
        for entry in BDP_ENTRIES {
            assert!(content.contains(entry), "Missing entry: {}", entry);
        }
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
        assert!(!content_after.contains(".bdp/cache/"));
    }

    #[test]
    fn test_format_bdp_section() {
        let section = format_bdp_section();
        assert!(section.starts_with(BDP_SECTION_MARKER));
        assert!(section.ends_with('\n'));
        for entry in BDP_ENTRIES {
            assert!(section.contains(entry));
        }
    }
}
