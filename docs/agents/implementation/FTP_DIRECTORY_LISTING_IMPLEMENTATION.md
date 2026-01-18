# FTP Directory Listing Implementation

## Overview
Replaced the mock implementation in `version_discovery.rs` with actual FTP directory listing functionality to discover UniProt previous releases from the FTP server.

## Changes Made

### 1. Added FTP Directory Listing to `UniProtFtp` (ftp.rs)

Added two new methods to the `UniProtFtp` struct:

#### `list_directories(&self, path: &str) -> Result<Vec<String>>`
- **Purpose**: List directories in a given FTP path
- **Features**:
  - Async wrapper using tokio spawn_blocking for synchronous FTP operations
  - Automatic retry logic (3 attempts with exponential backoff)
  - Comprehensive error logging with tracing
  - Returns only directory names (not full paths)
- **Location**: Lines 230-277 in `crates/bdp-server/src/ingest/uniprot/ftp.rs`

#### `list_directories_sync(config: &UniProtFtpConfig, path: &str) -> Result<Vec<String>>`
- **Purpose**: Synchronous FTP LIST implementation
- **Features**:
  - Connects to FTP server
  - Authenticates with configured credentials
  - Issues FTP LIST command
  - Parses FTP LIST output to extract directory names
  - Filters for directories only (entries starting with 'd')
  - Graceful connection cleanup
- **Location**: Lines 279-334 in `crates/bdp-server/src/ingest/uniprot/ftp.rs`

### 2. Replaced Mock in `version_discovery.rs`

Replaced the `list_previous_releases()` method:

#### Before (Lines 127-145)
```rust
/// List previous release directories (mock for now - would use FTP LIST command)
async fn list_previous_releases(&self) -> Result<Vec<String>> {
    // TODO: Implement actual FTP directory listing
    // For now, return a reasonable range
    let mut dirs = Vec::new();

    // Generate last 6 months of releases (assuming monthly)
    let current_year = 2025;
    let current_month = 1;

    for offset in 0..6 {
        let month = (current_month as i32 - offset).rem_euclid(12) + 1;
        let year = current_year - (current_month as i32 - offset - 1) / 12;

        dirs.push(format!("release-{:04}_{:02}", year, month));
    }

    Ok(dirs)
}
```

#### After (Lines 127-149)
```rust
/// List previous release directories using FTP LIST command
async fn list_previous_releases(&self) -> Result<Vec<String>> {
    let path = format!("{}/previous_releases", self.config.ftp_base_path);

    // Use FTP LIST command to get actual directory listing
    let directories = self
        .ftp
        .list_directories(&path)
        .await
        .context("Failed to list previous releases from FTP")?;

    // Filter to only include directories matching the release pattern
    let release_pattern = Regex::new(r"^release-\d{4}_\d{2}$")?;
    let mut releases: Vec<String> = directories
        .into_iter()
        .filter(|name| release_pattern.is_match(name))
        .collect();

    // Sort chronologically (oldest to newest)
    releases.sort();

    Ok(releases)
}
```

### 3. Added Unit Tests

Added comprehensive unit tests to verify FTP LIST parsing logic:

- **`test_parse_ftp_list_entries`**: Tests parsing of typical FTP LIST output format
  - Validates extraction of directory names
  - Ensures files are filtered out
  - Verifies only directories are returned

- **`test_parse_empty_ftp_list`**: Tests handling of empty FTP LIST responses

**Location**: Lines 424-467 in `crates/bdp-server/src/ingest/uniprot/ftp.rs`

### 4. Created Integration Test Example

Created `test_ftp_listing.rs` example program for manual testing:
- **Purpose**: Manually verify FTP connectivity and directory listing
- **Features**:
  - Connects to actual UniProt FTP server
  - Lists all available versions
  - Displays version information in tabular format
  - Provides helpful error messages
- **Usage**: `cargo run --example test_ftp_listing`
- **Location**: `crates/bdp-server/examples/test_ftp_listing.rs`

## Implementation Details

### FTP LIST Format Parsing
The implementation handles standard Unix-style FTP LIST output:
```
drwxr-xr-x   2 ftp      ftp          4096 Jan 15 12:00 release-2025_01
drwxr-xr-x   2 ftp      ftp          4096 Dec 15 12:00 release-2024_12
-rw-r--r--   1 ftp      ftp          1234 Jan 15 12:00 README.txt
```

The parser:
1. Splits each line by whitespace
2. Checks if first field starts with 'd' (directory indicator)
3. Extracts the last field as the directory name
4. Filters out files and other non-directory entries

### Error Handling
The implementation includes robust error handling:

1. **Connection Errors**: Retries up to 3 times with exponential backoff
2. **FTP Errors**: Wrapped with context for better error messages
3. **Parsing Errors**: Regex validation ensures only valid release directories are processed
4. **Logging**: Comprehensive debug/info/warn logging at all stages

### FTP Connection Details
- **Server**: ftp.uniprot.org:21
- **Path**: /pub/databases/uniprot/previous_releases/
- **Authentication**: Anonymous (anonymous/anonymous)
- **Mode**: Binary transfer
- **Timeout**: Controlled by tokio task spawning

## Testing Strategy

### Unit Tests
- FTP LIST parsing logic is tested independently
- Mock FTP responses are used to verify parsing correctness
- Edge cases (empty lists, mixed files/directories) are covered

### Integration Testing
- `test_ftp_listing.rs` example can be run manually
- Connects to real UniProt FTP server
- Verifies end-to-end functionality

### Running Tests
```bash
# Run unit tests
cargo test --package bdp-server --lib ingest::uniprot::ftp

# Run integration test (manual)
cargo run --example test_ftp_listing
```

## Benefits

1. **Real Data**: Now uses actual FTP server data instead of hardcoded mock
2. **Dynamic Discovery**: Automatically discovers all available previous releases
3. **Reliability**: Retry logic handles transient network issues
4. **Observability**: Comprehensive logging for debugging
5. **Maintainability**: Clean separation of async/sync FTP operations
6. **Testability**: Unit tests verify parsing logic independently

## Dependencies

No new dependencies were added. The implementation uses existing crates:
- `suppaftp`: For FTP operations
- `tokio`: For async/await and spawn_blocking
- `tracing`: For logging
- `anyhow`: For error handling
- `regex`: For pattern matching

## Future Enhancements

Potential improvements for future iterations:
1. Add caching to reduce FTP connections
2. Support for FTP MLSD command (more structured directory listing)
3. Parallel release notes fetching
4. Configurable timeout values
5. FTP connection pooling
