//! Search command implementation
//!
//! Interactive search for data sources and tools in the BDP registry.
//! Features fzf-style multi-select with fuzzy filtering, bulk actions,
//! and smart pipe detection.

use std::io::{self, IsTerminal};

use colored::Colorize;
use tracing::{debug, warn};

use crate::{
    api::{
        client::ApiClient,
        types::{SearchResponse, SearchResult},
    },
    cache::search_cache::{SearchCache, SearchFilters},
    error::{CliError, Result},
};

/// Run the search command
///
/// # Arguments
///
/// * `query` - Search query terms (will be joined with spaces)
/// * `org` - Optional filter by organization slug
/// * `entry_type` - Optional filter by entry type (data_source, tool,
///   organization)
/// * `source_type` - Optional filter by source type (protein, genome, etc.)
/// * `format` - Output format (interactive, compact, table, json)
/// * `no_interactive` - Force non-interactive mode
/// * `limit` - Number of results per page
/// * `page` - Page number
/// * `server_url` - BDP server URL
#[allow(clippy::too_many_arguments)]
pub async fn run(
    query: Vec<String>,
    org: Option<String>,
    entry_type: Vec<String>,
    source_type: Vec<String>,
    format: String,
    no_interactive: bool,
    limit: i32,
    page: i32,
    server_url: String,
) -> Result<()> {
    // Join query terms with spaces
    let query_str = query.join(" ");

    if query_str.trim().is_empty() {
        return Err(CliError::config("Search query cannot be empty"));
    }

    debug!(
        query = %query_str,
        org = ?org,
        entry_type = ?entry_type,
        source_type = ?source_type,
        format = %format,
        limit = limit,
        page = page,
        "Starting search"
    );

    // Validate pagination parameters
    if !(1..=100).contains(&limit) {
        return Err(CliError::config("Limit must be between 1 and 100"));
    }

    if page < 1 {
        return Err(CliError::config("Page must be greater than 0"));
    }

    // Create API client
    let client = ApiClient::new(server_url)?;

    // Determine if we should use interactive mode
    let use_interactive = should_use_interactive(&format, no_interactive);

    // Parse filters
    let type_filter = if entry_type.is_empty() {
        None
    } else {
        Some(entry_type)
    };

    let source_type_filter = if source_type.is_empty() {
        None
    } else {
        Some(source_type)
    };

    // Execute search with caching and retries
    println!("Searching for '{}'...", query_str);

    // For interactive mode, fetch a larger page to maximize local filtering
    // candidates
    let fetch_limit = if use_interactive { 100 } else { limit };

    // Create cache filters
    let cache_filters = SearchFilters {
        type_filter: type_filter.clone(),
        source_type_filter: source_type_filter.clone(),
        organism: None,
        format: None,
        org: org.clone(),
    };

    let search_results = execute_search_with_cache(
        &client,
        &query_str,
        type_filter,
        source_type_filter,
        page,
        fetch_limit,
        &cache_filters,
    )
    .await?;

    // Apply client-side org filter
    let search_results = if let Some(ref org_filter) = org {
        SearchResponse {
            results: search_results
                .results
                .into_iter()
                .filter(|r| r.organization_slug.eq_ignore_ascii_case(org_filter))
                .collect(),
            ..search_results
        }
    } else {
        search_results
    };

    if search_results.results.is_empty() {
        handle_empty_results(&query_str)?;
        return Ok(());
    }

    // Display results based on mode
    if use_interactive {
        display_interactive_multiselect(search_results).await?;
    } else {
        display_non_interactive(search_results, &format)?;
    }

    Ok(())
}

/// Determine if we should use interactive mode
fn should_use_interactive(format: &str, no_interactive: bool) -> bool {
    if no_interactive {
        return false;
    }

    if format != "interactive" {
        return false;
    }

    // Check if stdout is a TTY
    io::stdout().is_terminal()
}

/// Execute search with caching and retry logic
async fn execute_search_with_cache(
    client: &ApiClient,
    query: &str,
    type_filter: Option<Vec<String>>,
    source_type_filter: Option<Vec<String>>,
    page: i32,
    limit: i32,
    cache_filters: &SearchFilters,
) -> Result<SearchResponse> {
    // Initialize cache
    let cache_dir = if let Ok(custom_cache) = std::env::var("BDP_CACHE_DIR") {
        std::path::PathBuf::from(custom_cache)
    } else {
        dirs::cache_dir()
            .ok_or_else(|| CliError::config("Cannot find cache directory"))?
            .join("bdp")
    };
    std::fs::create_dir_all(&cache_dir)?;
    let cache_path = cache_dir.join("bdp.db");

    let cache = SearchCache::new(cache_path)?;
    cache.init()?;

    // Try to get from cache first
    if let Some(cached_response) = cache.get(query, cache_filters)? {
        debug!("Using cached search results");
        return Ok(cached_response);
    }

    // Cache miss - execute search
    let response =
        execute_search(client, query, type_filter, source_type_filter, page, limit).await?;

    // Store in cache
    if let Err(e) = cache.set(query, cache_filters, &response) {
        warn!(error = %e, "Failed to cache search results");
        // Don't fail the command if caching fails
    }

    Ok(response)
}

/// Execute search with retry logic
async fn execute_search(
    client: &ApiClient,
    query: &str,
    type_filter: Option<Vec<String>>,
    source_type_filter: Option<Vec<String>>,
    page: i32,
    limit: i32,
) -> Result<SearchResponse> {
    const MAX_RETRIES: u32 = 3;
    const INITIAL_BACKOFF_MS: u64 = 100;

    let mut attempt = 0;
    let mut last_error = None;

    while attempt < MAX_RETRIES {
        match client
            .search_with_filters(
                query,
                type_filter.clone(),
                source_type_filter.clone(),
                None, // organism filter
                None, // format filter
                Some(page),
                Some(limit),
            )
            .await
        {
            Ok(response) => {
                debug!(
                    results = response.results.len(),
                    total = response.total,
                    "Search successful"
                );
                return Ok(response);
            },
            Err(e) => {
                attempt += 1;
                last_error = Some(e);

                if attempt < MAX_RETRIES {
                    let backoff_ms = INITIAL_BACKOFF_MS * 2_u64.pow(attempt - 1);
                    warn!(attempt = attempt, backoff_ms = backoff_ms, "Search failed, retrying...");
                    tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                }
            },
        }
    }

    Err(last_error.unwrap_or_else(|| CliError::api("Search failed after retries".to_string())))
}

/// Handle empty search results with helpful suggestions
fn handle_empty_results(query: &str) -> Result<()> {
    println!("{}", "No results found".bold().red());
    println!();

    // Try to provide fuzzy suggestions
    let suggestions = find_similar_terms(query);
    if !suggestions.is_empty() {
        println!("{}", "Did you mean:".bold());
        for suggestion in suggestions {
            println!("  {} {}", "•".blue(), suggestion);
        }
        println!();
    }

    // Provide helpful tips
    println!("{}", "Try:".bold());
    println!("  {} Check your spelling", "•".blue());
    println!("  {} Use fewer keywords", "•".blue());
    println!(
        "  {} Browse all data sources: {}",
        "•".blue(),
        "bdp search --type data-source".cyan()
    );
    println!(
        "  {} Search in organizations: {}",
        "•".blue(),
        "bdp search <query> --type organization".cyan()
    );

    Ok(())
}

/// Find similar terms using fuzzy matching
fn find_similar_terms(query: &str) -> Vec<String> {
    // Common terms in bioinformatics that might match
    let common_terms = vec![
        "insulin",
        "insulin-like",
        "protein",
        "genome",
        "kinase",
        "transcription",
        "blast",
        "uniprot",
        "genbank",
        "refseq",
    ];

    let mut suggestions = Vec::new();
    for term in common_terms {
        let distance = strsim::levenshtein(query, term);
        if distance <= 3 && distance > 0 {
            suggestions.push(term.to_string());
        }
    }

    // Limit to top 3 suggestions
    suggestions.truncate(3);
    suggestions
}

/// Format a single search result as a display line for the MultiSelect picker.
///
/// Format: `org:slug -- Name [fmt1, fmt2] v1.0`
fn format_result_line(r: &SearchResult) -> String {
    let spec = format!("{}:{}", r.organization_slug, r.slug);
    let fmts = if r.available_formats.is_empty() {
        String::new()
    } else {
        format!(" [{}]", r.available_formats.join(", "))
    };
    let ver = r
        .latest_version
        .as_deref()
        .map(|v| format!(" v{v}"))
        .unwrap_or_default();
    format!("{spec} -- {}{fmts}{ver}", r.name)
}

/// Display results in interactive multi-select mode.
///
/// Users can fuzzy-filter results by typing, toggle selections with Space,
/// confirm with Enter, and then choose bulk actions.
async fn display_interactive_multiselect(results: SearchResponse) -> Result<()> {
    use inquire::MultiSelect;

    // Show result count summary
    println!();
    println!("{} Found {} results", "✓".green(), results.results.len());
    println!();

    if results.results.is_empty() {
        return Ok(());
    }

    // Build display lines (one per result)
    let display_lines: Vec<String> = results.results.iter().map(format_result_line).collect();

    // Present multi-select picker
    let selections = MultiSelect::new(
        "Select sources (type to filter, Space to toggle, Enter to confirm):",
        display_lines.clone(),
    )
    .with_page_size(15)
    .with_vim_mode(true)
    .prompt();

    let selected_labels = match selections {
        Ok(sel) if sel.is_empty() => {
            println!("No sources selected.");
            return Ok(());
        },
        Ok(sel) => sel,
        Err(_) => {
            // User cancelled (Esc / Ctrl+C)
            return Ok(());
        },
    };

    // Map selected labels back to SearchResult refs
    let selected_results: Vec<&SearchResult> = selected_labels
        .iter()
        .filter_map(|label| {
            display_lines
                .iter()
                .position(|l| l == label)
                .map(|idx| &results.results[idx])
        })
        .collect();

    if selected_results.is_empty() {
        return Ok(());
    }

    // Show what was selected
    println!();
    println!(
        "{} {} selected:",
        selected_results.len().to_string().bold(),
        if selected_results.len() == 1 {
            "source"
        } else {
            "sources"
        }
    );
    for r in &selected_results {
        let ver = r.latest_version.as_deref().unwrap_or("latest");
        println!("  {} {}:{}@{}", "•".blue(), r.organization_slug, r.slug, ver);
    }
    println!();

    // Show bulk actions menu
    show_bulk_actions(&selected_results).await?;

    Ok(())
}

/// Show the bulk actions menu after multi-select.
async fn show_bulk_actions(selected: &[&SearchResult]) -> Result<()> {
    use inquire::Select;

    let actions = vec![
        "Add all to manifest (bdp.yml)",
        "Copy all specs to clipboard",
        "View details",
        "Cancel",
    ];

    let action = Select::new("What would you like to do?", actions).prompt();

    match action {
        Ok("Add all to manifest (bdp.yml)") => {
            add_multiple_to_manifest(selected)?;
        },
        Ok("Copy all specs to clipboard") => {
            copy_multiple_to_clipboard(selected)?;
        },
        Ok("View details") => {
            for r in selected {
                display_result_details(r)?;
            }
        },
        Ok("Cancel") | Err(_) => {},
        _ => {},
    }

    Ok(())
}

/// Add multiple selected results to the manifest.
///
/// For each result, builds the spec with format suffix (prompting user if
/// there are multiple formats). Skips duplicates, saves the manifest once
/// at the end.
fn add_multiple_to_manifest(selected: &[&SearchResult]) -> Result<()> {
    use crate::manifest::Manifest;

    let manifest_path = find_manifest_file()?;
    let mut manifest = Manifest::load(&manifest_path)?;

    let mut added = 0usize;
    let mut skipped = 0usize;

    for result in selected {
        let spec = match build_manifest_spec(result) {
            Ok(s) => s,
            Err(e) => {
                println!(
                    "  {} Skipped {}:{} ({})",
                    "⚠".yellow(),
                    result.organization_slug,
                    result.slug,
                    e
                );
                skipped += 1;
                continue;
            },
        };

        if manifest.sources.contains(&spec) {
            println!("  {} Already in manifest: {}", "⚠".yellow(), spec.cyan());
            skipped += 1;
            continue;
        }

        manifest.add_source(spec.clone());
        println!("  {} {}", "✓".green(), spec.cyan());
        added += 1;
    }

    // Save once
    if added > 0 {
        manifest.save(&manifest_path)?;
    }

    println!();
    println!(
        "Added {} source{}, skipped {} (already in manifest or cancelled)",
        added,
        if added == 1 { "" } else { "s" },
        skipped,
    );

    Ok(())
}

/// Copy specs for all selected results to the clipboard.
fn copy_multiple_to_clipboard(selected: &[&SearchResult]) -> Result<()> {
    let specs: Vec<String> = selected
        .iter()
        .map(|r| {
            let ver = r.latest_version.as_deref().unwrap_or("latest");
            format!("{}:{}@{}", r.organization_slug, r.slug, ver)
        })
        .collect();

    let joined = specs.join("\n");
    match copy_to_clipboard(&joined) {
        Ok(()) => {
            println!("{} Copied {} specs to clipboard:", "✓".green(), specs.len());
            for s in &specs {
                println!("  {}", s.cyan());
            }
        },
        Err(e) => {
            println!("{} Failed to copy to clipboard: {}", "✗".red(), e);
            println!("Specs:");
            for s in &specs {
                println!("  {}", s.cyan());
            }
        },
    }
    Ok(())
}

/// Build the full source spec including format suffix.
/// Format: `org:slug-format@version` (e.g., `uniprot:P01308-fasta@1.0`)
///
/// If the source has multiple formats, prompts the user to choose.
/// If no formats are available, returns the spec without a format suffix.
fn build_manifest_spec(result: &SearchResult) -> Result<String> {
    use inquire::Select;

    let version = result.latest_version.as_deref().unwrap_or("latest");

    match result.available_formats.len() {
        0 => {
            // No formats known — return without format suffix
            Ok(format!("{}:{}@{}", result.organization_slug, result.slug, version))
        },
        1 => {
            let fmt = &result.available_formats[0];
            Ok(format!("{}:{}-{}@{}", result.organization_slug, result.slug, fmt, version))
        },
        _ => {
            let choice = Select::new(
                &format!("Which format for {}:{}?", result.organization_slug, result.slug),
                result.available_formats.clone(),
            )
            .prompt()
            .map_err(|_| CliError::config("Format selection cancelled"))?;
            Ok(format!("{}:{}-{}@{}", result.organization_slug, result.slug, choice, version))
        },
    }
}

/// Display detailed information about a search result
fn display_result_details(result: &SearchResult) -> Result<()> {
    use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Table};

    println!();
    println!("{}", "═".repeat(60).blue());
    println!("{}", format!("  {}", result.name).bold());
    println!("{}", "═".repeat(60).blue());
    println!();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS);

    table.add_row(vec!["ID", &result.id]);
    table.add_row(vec!["Organization", &result.organization_slug]);
    table.add_row(vec!["Name", &result.name]);
    table.add_row(vec!["Version", result.latest_version.as_deref().unwrap_or("latest")]);
    table.add_row(vec!["Formats", &result.available_formats.join(", ")]);
    table.add_row(vec!["Type", &result.entry_type]);

    if let Some(ref desc) = result.description {
        table.add_row(vec!["Description", desc]);
    }

    println!("{}", table);
    println!();

    // Spec for copying
    let spec = format!(
        "{}:{}@{}",
        result.organization_slug,
        result.slug,
        result.latest_version.as_deref().unwrap_or("latest")
    );
    println!("Spec: {}", spec.cyan());
    println!();

    Ok(())
}

/// Display results in non-interactive mode with smart pipe detection.
fn display_non_interactive(results: SearchResponse, format: &str) -> Result<()> {
    match format {
        "compact" => display_compact(&results),
        "table" => display_table(&results),
        "json" => display_json(&results),
        _ => {
            // Default: if piped, use compact specs; if TTY, use table
            if io::stdout().is_terminal() {
                display_table(&results)
            } else {
                display_compact(&results)
            }
        },
    }
}

/// Display results in compact format.
///
/// When piped (non-TTY): bare specs only (`org:slug@version`, one per line).
/// When TTY: rich format (`org:slug -- Name [formats] vX.Y`).
fn display_compact(results: &SearchResponse) -> Result<()> {
    let is_tty = io::stdout().is_terminal();

    for result in &results.results {
        if is_tty {
            println!("{}", format_result_line(result));
        } else {
            // Bare spec for piping
            println!(
                "{}:{}@{}",
                result.organization_slug,
                result.slug,
                result.latest_version.as_deref().unwrap_or("latest")
            );
        }
    }
    Ok(())
}

/// Display results in table format
fn display_table(results: &SearchResponse) -> Result<()> {
    use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Table};

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec!["Source", "Name", "Format", "Type", "Description"]);

    for result in &results.results {
        let source = format!("{}:{}", result.organization_slug, result.slug);
        let description = result
            .description
            .as_ref()
            .map(|d| truncate_string(d, 50))
            .unwrap_or_else(|| "-".to_string());

        table.add_row(vec![
            source,
            result.name.clone(),
            result.available_formats.join(", "),
            result.entry_type.clone(),
            description,
        ]);
    }

    println!();
    println!("{}", table);
    println!();
    println!(
        "Showing {} of {} results (page {}/{})",
        results.results.len(),
        results.total,
        results.page,
        (results.total as f64 / results.page_size as f64).ceil() as i32
    );

    Ok(())
}

/// Display results in JSON format
fn display_json(results: &SearchResponse) -> Result<()> {
    let json = serde_json::to_string_pretty(results)?;
    println!("{}", json);
    Ok(())
}

/// Truncate a string to a maximum length with ellipsis
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Add a source to the manifest (bdp.yml)
#[cfg(test)]
async fn add_to_manifest(spec: &str) -> Result<()> {
    use crate::manifest::Manifest;

    // Find bdp.yml in current directory or parent directories
    let manifest_path = find_manifest_file()?;

    // Load existing manifest
    let mut manifest = Manifest::load(&manifest_path)?;

    // Check if source already exists
    if manifest.sources.contains(&spec.to_string()) {
        return Err(CliError::config("Source already exists in manifest"));
    }

    // Add new source
    manifest.add_source(spec.to_string());

    // Save manifest
    manifest.save(&manifest_path)?;

    Ok(())
}

/// Find bdp.yml file in current or parent directories
fn find_manifest_file() -> Result<std::path::PathBuf> {
    let mut current_dir = std::env::current_dir()?;

    loop {
        let manifest_path = current_dir.join("bdp.yml");
        if manifest_path.exists() {
            return Ok(manifest_path);
        }

        // Try parent directory
        if let Some(parent) = current_dir.parent() {
            current_dir = parent.to_path_buf();
        } else {
            break;
        }
    }

    Err(CliError::config(
        "No bdp.yml found in current directory or parent directories. Run 'bdp init' first.",
    ))
}

/// Copy text to system clipboard
fn copy_to_clipboard(text: &str) -> Result<()> {
    use arboard::Clipboard;

    let mut clipboard = Clipboard::new()
        .map_err(|e| CliError::config(format!("Failed to access clipboard: {}", e)))?;

    clipboard
        .set_text(text)
        .map_err(|e| CliError::config(format!("Failed to copy to clipboard: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 8), "hello...");
        assert_eq!(truncate_string("hi", 5), "hi");
    }

    #[test]
    fn test_find_similar_terms() {
        let suggestions = find_similar_terms("insulinn");
        assert!(suggestions.contains(&"insulin".to_string()));

        let suggestions = find_similar_terms("protin");
        assert!(suggestions.contains(&"protein".to_string()));
    }

    #[test]
    fn test_should_use_interactive() {
        assert!(!should_use_interactive("table", false));
        assert!(!should_use_interactive("interactive", true));
        assert!(!should_use_interactive("json", false));
    }

    #[test]
    fn test_format_result_line() {
        let result = SearchResult {
            id: "test-id".to_string(),
            organization_slug: "uniprot".to_string(),
            slug: "P01308".to_string(),
            name: "Insulin".to_string(),
            description: Some("Insulin protein".to_string()),
            entry_type: "data_source".to_string(),
            source_type: Some("protein".to_string()),
            latest_version: Some("1.0".to_string()),
            external_version: None,
            available_formats: vec!["fasta".to_string(), "xml".to_string()],
            organism: None,
            rank: None,
        };

        let line = format_result_line(&result);
        assert!(line.contains("uniprot:P01308"));
        assert!(line.contains("Insulin"));
        assert!(line.contains("[fasta, xml]"));
        assert!(line.contains("v1.0"));
    }

    #[test]
    fn test_format_result_line_no_formats() {
        let result = SearchResult {
            id: "test-id".to_string(),
            organization_slug: "ncbi".to_string(),
            slug: "NC_000001".to_string(),
            name: "Chromosome 1".to_string(),
            description: None,
            entry_type: "data_source".to_string(),
            source_type: None,
            latest_version: None,
            external_version: None,
            available_formats: vec![],
            organism: None,
            rank: None,
        };

        let line = format_result_line(&result);
        assert!(line.contains("ncbi:NC_000001"));
        assert!(line.contains("Chromosome 1"));
        assert!(!line.contains('['));
        assert!(!line.contains('v'));
    }

    #[tokio::test]
    #[serial]
    async fn test_add_to_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("bdp.yml");

        // Create a test manifest
        let manifest_content = r#"
project:
  name: test-project
  version: 0.1.0

sources: []
tools: []
"#;
        std::fs::write(&manifest_path, manifest_content).unwrap();

        // Save original directory (if available)
        let original_dir = std::env::current_dir().ok();

        // Change to temp directory
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Add source to manifest
        let result = add_to_manifest("uniprot:P01308@1.0").await;
        assert!(result.is_ok());

        // Verify source was added
        let manifest = crate::manifest::Manifest::load(&manifest_path).unwrap();
        assert!(manifest.sources.contains(&"uniprot:P01308@1.0".to_string()));
        assert_eq!(manifest.sources.len(), 1);

        // Try adding duplicate - should return error
        let result = add_to_manifest("uniprot:P01308@1.0").await;
        assert!(result.is_err());

        // Verify still only one source
        let manifest = crate::manifest::Manifest::load(&manifest_path).unwrap();
        assert_eq!(manifest.sources.len(), 1);

        // Add a different source
        let result = add_to_manifest("genbank:NC_000001@2.0").await;
        assert!(result.is_ok());

        // Verify two sources now
        let manifest = crate::manifest::Manifest::load(&manifest_path).unwrap();
        assert_eq!(manifest.sources.len(), 2);

        // Restore original directory (if we saved one)
        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(dir);
        }
    }

    #[test]
    #[serial]
    fn test_find_manifest_file() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("bdp.yml");

        // Create a test manifest
        std::fs::write(&manifest_path, "test").unwrap();

        // Save original directory (if available)
        let original_dir = std::env::current_dir().ok();

        // Change to temp directory
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Should find manifest
        let result = find_manifest_file();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), manifest_path);

        // Restore original directory (if we saved one)
        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(dir);
        }
    }

    #[test]
    #[serial]
    fn test_find_manifest_file_not_found() {
        let temp_dir = TempDir::new().unwrap();

        // Save original directory (if available)
        let original_dir = std::env::current_dir().ok();

        // Change to temp directory (no manifest)
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Should not find manifest
        let result = find_manifest_file();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No bdp.yml found"));

        // Restore original directory (if we saved one)
        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(dir);
        }
    }

    #[test]
    fn test_copy_to_clipboard() {
        // Test clipboard functionality
        let result = copy_to_clipboard("test-spec");

        // Clipboard might not be available in CI/test environment
        // So we just check that the function doesn't panic
        let _ = result;
    }
}
