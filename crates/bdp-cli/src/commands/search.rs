//! Search command implementation
//!
//! Interactive search for data sources and tools in the BDP registry.

use crate::api::client::ApiClient;
use crate::cache::search_cache::{SearchCache, SearchFilters};
use crate::error::{CliError, Result};
use colored::Colorize;
use std::io::{self, IsTerminal};
use tracing::{debug, info, warn};

/// Run the search command
///
/// # Arguments
///
/// * `query` - Search query terms (will be joined with spaces)
/// * `entry_type` - Optional filter by entry type (data_source, tool, organization)
/// * `source_type` - Optional filter by source type (protein, genome, etc.)
/// * `format` - Output format (interactive, compact, table, json)
/// * `no_interactive` - Force non-interactive mode
/// * `limit` - Number of results per page
/// * `page` - Page number
/// * `server_url` - BDP server URL
#[allow(clippy::too_many_arguments)]
pub async fn run(
    query: Vec<String>,
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
        entry_type = ?entry_type,
        source_type = ?source_type,
        format = %format,
        limit = limit,
        page = page,
        "Starting search"
    );

    // Validate pagination parameters
    if limit < 1 || limit > 100 {
        return Err(CliError::config("Limit must be between 1 and 100"));
    }

    if page < 1 {
        return Err(CliError::config("Page must be greater than 0"));
    }

    // Create API client
    let client = ApiClient::new(server_url.clone())?;

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
    info!("Searching for '{}'...", query_str);

    // Create cache filters
    let cache_filters = SearchFilters {
        type_filter: type_filter.clone(),
        source_type_filter: source_type_filter.clone(),
        organism: None,
        format: None,
    };

    let search_results = execute_search_with_cache(
        &client,
        &query_str,
        type_filter.clone(),
        source_type_filter.clone(),
        page,
        limit,
        &cache_filters,
    )
    .await?;

    if search_results.results.is_empty() {
        handle_empty_results(&query_str)?;
        return Ok(());
    }

    // Display results based on mode
    if use_interactive {
        let state = InteractiveState {
            query: query_str.clone(),
            type_filter: type_filter.clone(),
            source_type_filter: source_type_filter.clone(),
            limit,
            current_page: page,
            server_url,
        };
        display_interactive(search_results, state).await?;
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
) -> Result<crate::api::types::SearchResponse> {
    // Initialize cache
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| CliError::config("Cannot find cache directory"))?
        .join("bdp");
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
) -> Result<crate::api::types::SearchResponse> {
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
                    warn!(
                        attempt = attempt,
                        backoff_ms = backoff_ms,
                        "Search failed, retrying..."
                    );
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

/// Interactive mode state for pagination
struct InteractiveState {
    query: String,
    type_filter: Option<Vec<String>>,
    source_type_filter: Option<Vec<String>>,
    limit: i32,
    current_page: i32,
    server_url: String,
}

/// Display results in interactive mode
async fn display_interactive(
    results: crate::api::types::SearchResponse,
    state: InteractiveState,
) -> Result<()> {
    use inquire::Select;

    let mut current_results = results;
    let mut current_page = state.current_page;

    loop {
        // Display search summary
        println!();
        println!(
            "{} Found {} results for your search",
            "✓".green(),
            current_results.total
        );
        let total_pages = (current_results.total as f64 / current_results.page_size as f64).ceil() as i32;
        println!(
            "  Showing page {}/{}",
            current_page,
            total_pages
        );
        println!();

        // Create selection options with formatted display
        let options: Vec<String> = current_results
            .results
            .iter()
            .map(|r| {
                let spec = format!("{}:{}@{}", r.organization, r.name, r.version);
                let desc = r
                    .description
                    .as_ref()
                    .map(|d| truncate_string(d, 60))
                    .unwrap_or_else(|| "No description".to_string());
                format!("{} - {}", spec.cyan(), desc)
            })
            .collect();

        // Add pagination options if needed
        let mut all_options = options.clone();
        if current_page > 1 {
            all_options.push(format!("{}", "← Previous page".yellow()));
        }
        if current_page < total_pages {
            all_options.push(format!("{}", "→ Next page".yellow()));
        }
        all_options.push(format!("{}", "✕ Exit".red()));

        // Show selection menu
        let selection = Select::new("Select a data source:", all_options.clone())
            .with_page_size(15)
            .prompt();

        match selection {
            Ok(selected) => {
                // Check for special options
                if selected.contains("✕ Exit") {
                    break;
                } else if selected.contains("← Previous page") {
                    // Fetch previous page
                    current_page -= 1;
                    println!("{}", "Loading previous page...".cyan());
                    let client = ApiClient::new(state.server_url.clone())?;
                    let cache_filters = SearchFilters {
                        type_filter: state.type_filter.clone(),
                        source_type_filter: state.source_type_filter.clone(),
                        organism: None,
                        format: None,
                    };
                    current_results = execute_search_with_cache(
                        &client,
                        &state.query,
                        state.type_filter.clone(),
                        state.source_type_filter.clone(),
                        current_page,
                        state.limit,
                        &cache_filters,
                    ).await?;
                    continue;
                } else if selected.contains("→ Next page") {
                    // Fetch next page
                    current_page += 1;
                    println!("{}", "Loading next page...".cyan());
                    let client = ApiClient::new(state.server_url.clone())?;
                    let cache_filters = SearchFilters {
                        type_filter: state.type_filter.clone(),
                        source_type_filter: state.source_type_filter.clone(),
                        organism: None,
                        format: None,
                    };
                    current_results = execute_search_with_cache(
                        &client,
                        &state.query,
                        state.type_filter.clone(),
                        state.source_type_filter.clone(),
                        current_page,
                        state.limit,
                        &cache_filters,
                    ).await?;
                    continue;
                }

                // Find the selected result
                let selected_index = all_options
                    .iter()
                    .position(|o| o == &selected)
                    .ok_or_else(|| CliError::config("Invalid selection"))?;

                if selected_index < current_results.results.len() {
                    let result = &current_results.results[selected_index];
                    show_result_actions(result).await?;
                }
            }
            Err(_) => {
                // User cancelled (Ctrl+C or ESC)
                break;
            }
        }
    }

    Ok(())
}

/// Show action menu for a selected result
async fn show_result_actions(result: &crate::api::types::SearchResult) -> Result<()> {
    use inquire::Select;

    let spec = format!("{}:{}@{}", result.organization, result.name, result.version);

    loop {
        println!();
        println!("{}", format!("Selected: {}", spec).bold());
        println!();

        let actions = vec![
            "📋 View details",
            "➕ Add to manifest (bdp.yml)",
            "📝 Copy spec to clipboard",
            "← Back to results",
        ];

        let action = Select::new("What would you like to do?", actions)
            .prompt();

        match action {
            Ok("📋 View details") => {
                display_result_details(result)?;
            }
            Ok("➕ Add to manifest (bdp.yml)") => {
                match add_to_manifest(&spec).await {
                    Ok(()) => {
                        println!("{} Added to manifest: {}", "✓".green(), spec.cyan());
                    }
                    Err(e) => {
                        println!("{} Failed to add to manifest: {}", "✗".red(), e);
                        println!("You can manually add to bdp.yml:");
                        println!("  sources:");
                        println!("    - spec: \"{}\"", spec.cyan());
                    }
                }
            }
            Ok("📝 Copy spec to clipboard") => {
                match copy_to_clipboard(&spec) {
                    Ok(()) => {
                        println!("{} Copied to clipboard: {}", "✓".green(), spec.cyan());
                    }
                    Err(e) => {
                        println!("{} Failed to copy to clipboard: {}", "✗".red(), e);
                        println!("Spec: {}", spec.cyan());
                    }
                }
            }
            Ok("← Back to results") | Err(_) => {
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Display detailed information about a search result
fn display_result_details(result: &crate::api::types::SearchResult) -> Result<()> {
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
    table.add_row(vec!["Organization", &result.organization]);
    table.add_row(vec!["Name", &result.name]);
    table.add_row(vec!["Version", &result.version]);
    table.add_row(vec!["Format", &result.format]);
    table.add_row(vec!["Type", &result.entry_type]);

    if let Some(ref desc) = result.description {
        table.add_row(vec!["Description", desc]);
    }

    println!("{}", table);
    println!();

    // Spec for copying
    let spec = format!("{}:{}@{}", result.organization, result.name, result.version);
    println!("{}", format!("Spec: {}", spec.cyan()));
    println!();

    Ok(())
}

/// Display results in non-interactive mode
fn display_non_interactive(
    results: crate::api::types::SearchResponse,
    format: &str,
) -> Result<()> {
    match format {
        "compact" => display_compact(&results),
        "table" => display_table(&results),
        "json" => display_json(&results),
        _ => {
            // Default to table for non-interactive
            display_table(&results)
        },
    }
}

/// Display results in compact format (one per line)
fn display_compact(results: &crate::api::types::SearchResponse) -> Result<()> {
    for result in &results.results {
        println!(
            "{}:{}@{}",
            result.organization, result.name, result.version
        );
    }
    Ok(())
}

/// Display results in table format
fn display_table(results: &crate::api::types::SearchResponse) -> Result<()> {
    use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Table};

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec!["Source", "Name", "Format", "Type", "Description"]);

    for result in &results.results {
        let source = format!("{}:{}", result.organization, result.name);
        let description = result
            .description
            .as_ref()
            .map(|d| truncate_string(d, 50))
            .unwrap_or_else(|| "-".to_string());

        table.add_row(vec![
            source,
            result.name.clone(),
            result.format.clone(),
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
fn display_json(results: &crate::api::types::SearchResponse) -> Result<()> {
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
        .map_err(|e| CliError::config(&format!("Failed to access clipboard: {}", e)))?;

    clipboard
        .set_text(text)
        .map_err(|e| CliError::config(&format!("Failed to copy to clipboard: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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

    #[tokio::test]
    async fn test_add_to_manifest() {
        use std::sync::Mutex;
        // Use a mutex to ensure this test doesn't run in parallel with others that change directory
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let _guard = TEST_LOCK.lock().unwrap();

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

        // Save original directory
        let original_dir = std::env::current_dir().unwrap();

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

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_find_manifest_file() {
        use std::sync::Mutex;
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let _guard = TEST_LOCK.lock().unwrap();

        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("bdp.yml");

        // Create a test manifest
        std::fs::write(&manifest_path, "test").unwrap();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Should find manifest
        let result = find_manifest_file();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), manifest_path);

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_find_manifest_file_not_found() {
        use std::sync::Mutex;
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let _guard = TEST_LOCK.lock().unwrap();

        let temp_dir = TempDir::new().unwrap();

        // Change to temp directory (no manifest)
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Should not find manifest
        let result = find_manifest_file();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No bdp.yml found"));

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
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
