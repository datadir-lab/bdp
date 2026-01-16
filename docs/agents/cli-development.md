# CLI Development Guide

Guide for developing the `bdp` CLI tool in Rust using clap.

## BDP Command Philosophy

BDP manages bioinformatics **data sources** (proteins, genomes, databases) and **tools** (aligners, assemblers, etc.), not traditional software packages. Commands follow this pattern:

```bash
bdp init                              # Initialize project
bdp source add uniprot:P12345@1.0     # Add protein from UniProt
bdp source add ncbi:genome/GCA_000001405.29  # Add reference genome
bdp tool add samtools@1.18            # Add bioinformatics tool
bdp lock                              # Lock all dependencies
bdp sync                              # Download and verify sources
```

## Project Structure

```
crates/bdp-cli/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs
│   ├── commands/         # CLI commands
│   │   ├── mod.rs
│   │   ├── init.rs       # bdp init
│   │   ├── source.rs     # bdp source add/remove/list
│   │   ├── tool.rs       # bdp tool add/remove/list
│   │   ├── lock.rs       # bdp lock
│   │   ├── sync.rs       # bdp sync (download sources)
│   │   └── env.rs        # bdp env
│   ├── sources/          # Source providers
│   │   ├── mod.rs
│   │   ├── uniprot.rs    # UniProt protein database
│   │   ├── ncbi.rs       # NCBI genomes, sequences
│   │   ├── pdb.rs        # Protein Data Bank
│   │   └── ensembl.rs    # Ensembl genomes
│   ├── tools/            # Tool management
│   │   ├── mod.rs
│   │   └── registry.rs   # Tool registry client
│   ├── config.rs         # Configuration management
│   ├── manifest.rs       # bdp.toml parsing
│   └── lockfile.rs       # bdp.lock management
└── tests/
```

## CLI Structure with clap

### main.rs

```rust
use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "bdp")]
#[command(about = "Bioinformatics Dependencies Platform", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new BDP project
    Init {
        /// Project name
        #[arg(default_value = ".")]
        path: String,
    },

    /// Manage bioinformatics data sources
    #[command(subcommand)]
    Source(SourceCommands),

    /// Manage bioinformatics tools
    #[command(subcommand)]
    Tool(ToolCommands),

    /// Generate or update lock file
    Lock {
        /// Update all dependencies
        #[arg(long)]
        update: bool,
    },

    /// Download and verify all sources and tools
    Sync {
        /// Force re-download even if cached
        #[arg(long)]
        force: bool,
    },

    /// Environment management
    #[command(subcommand)]
    Env(EnvCommands),
}

#[derive(Subcommand)]
enum SourceCommands {
    /// Add a data source (uniprot:P12345@1.0, ncbi:genome/GCA_000001405.29)
    Add {
        /// Source specification (provider:identifier@version)
        source: String,
    },

    /// Remove a data source
    Remove {
        /// Source name or identifier
        source: String,
    },

    /// List all sources in project
    List,

    /// Show source details
    Info {
        /// Source name or identifier
        source: String,
    },
}

#[derive(Subcommand)]
enum ToolCommands {
    /// Add a bioinformatics tool
    Add {
        /// Tool name and version (samtools@1.18)
        tool: String,
    },

    /// Remove a tool
    Remove {
        /// Tool name
        tool: String,
    },

    /// List all tools in project
    List,

    /// Search available tools
    Search {
        /// Search query
        query: String,
    },
}

#[derive(Subcommand)]
enum EnvCommands {
    /// Create environment snapshot
    Create {
        /// Environment name
        name: String,
    },

    /// List saved environments
    List,

    /// Restore from environment
    Restore {
        /// Environment ID or name
        id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    }

    match cli.command {
        Commands::Init { path } => {
            bdp_cli::commands::init::run(&path).await?;
        }
        Commands::Source(cmd) => {
            bdp_cli::commands::source::run(cmd).await?;
        }
        Commands::Tool(cmd) => {
            bdp_cli::commands::tool::run(cmd).await?;
        }
        Commands::Lock { update } => {
            bdp_cli::commands::lock::run(update).await?;
        }
        Commands::Sync { force } => {
            bdp_cli::commands::sync::run(force).await?;
        }
        Commands::Env(env_cmd) => {
            bdp_cli::commands::env::run(env_cmd).await?;
        }
    }

    Ok(())
}
```

## Manifest Parsing

### manifest.rs

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub project: ProjectMetadata,
    #[serde(default)]
    pub sources: HashMap<String, SourceSpec>,
    #[serde(default)]
    pub tools: HashMap<String, String>,  // tool_name -> version
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SourceSpec {
    /// Simple string format: "uniprot:P12345@1.0"
    Simple(String),
    /// Detailed format with options
    Detailed {
        provider: String,  // "uniprot", "ncbi", "pdb", "ensembl"
        identifier: String,
        version: String,
        #[serde(default)]
        options: HashMap<String, String>,
    },
}

impl Manifest {
    /// Load manifest from bdp.toml
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .context("Failed to read bdp.toml")?;

        toml::from_str(&content)
            .context("Failed to parse bdp.toml")
    }

    /// Create new manifest
    pub fn new(name: String, version: String) -> Self {
        Self {
            project: ProjectMetadata {
                name,
                version,
                description: None,
                authors: vec![],
                keywords: vec![],
            },
            sources: HashMap::new(),
            tools: HashMap::new(),
        }
    }

    /// Save manifest to file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize manifest")?;

        std::fs::write(path.as_ref(), content)
            .context("Failed to write bdp.toml")?;

        Ok(())
    }

    /// Add a source to the manifest
    pub fn add_source(&mut self, name: String, spec: SourceSpec) {
        self.sources.insert(name, spec);
    }

    /// Add a tool to the manifest
    pub fn add_tool(&mut self, name: String, version: String) {
        self.tools.insert(name, version);
    }
}

/// Example bdp.toml:
/// ```toml
/// [project]
/// name = "my-analysis"
/// version = "0.1.0"
/// description = "Protein structure analysis pipeline"
/// authors = ["Your Name <you@example.com>"]
/// keywords = ["protein", "structure"]
///
/// [sources]
/// insulin = "uniprot:P01308@1.0"
/// hemoglobin = { provider = "pdb", identifier = "1A3N", version = "latest" }
/// human_genome = "ncbi:genome/GCA_000001405.29"
///
/// [tools]
/// samtools = "1.18.0"
/// bwa = "0.7.17"
/// blast = "2.14.0"
/// ```
```

## Commands Implementation

### commands/init.rs

```rust
use anyhow::{Context, Result};
use dialoguer::Input;
use console::style;
use std::path::Path;
use crate::manifest::Manifest;

pub async fn run(path: &str) -> Result<()> {
    let path = Path::new(path);
    let bdp_toml = path.join("bdp.toml");

    if bdp_toml.exists() {
        anyhow::bail!("bdp.toml already exists in {}", path.display());
    }

    println!("{}", style("Initializing BDP project...").bold().green());

    // Interactive prompts
    let name: String = Input::new()
        .with_prompt("Project name")
        .default(
            path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("my-project")
                .to_string()
        )
        .interact_text()?;

    let version: String = Input::new()
        .with_prompt("Version")
        .default("0.1.0".to_string())
        .interact_text()?;

    let description: String = Input::new()
        .with_prompt("Description")
        .allow_empty(true)
        .interact_text()?;

    // Create manifest
    let mut manifest = Manifest::new(name.clone(), version);
    if !description.is_empty() {
        manifest.package.description = Some(description);
    }

    // Save to file
    manifest.save(&bdp_toml)?;

    println!("{} Created {}", style("✓").green(), bdp_toml.display());
    println!("\nRun {} to install dependencies", style("bdp install").cyan());

    Ok(())
}
```

### commands/install.rs

```rust
use anyhow::{Context, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use crate::{
    api_client::ApiClient,
    manifest::Manifest,
    lockfile::Lockfile,
    resolver::Resolver,
};

pub async fn run(packages: Vec<String>, _dev: bool) -> Result<()> {
    let manifest = Manifest::load("bdp.toml")
        .context("No bdp.toml found. Run 'bdp init' first.")?;

    let api_client = ApiClient::new()?;
    let resolver = Resolver::new(api_client.clone());

    println!("{}", style("Resolving dependencies...").bold());

    // If specific packages provided, add to dependencies
    let mut deps = manifest.dependencies.clone();
    for pkg in &packages {
        if !deps.contains_key(pkg) {
            deps.insert(pkg.clone(), "*".to_string());  // Latest version
        }
    }

    // Resolve dependency graph
    let resolved = resolver.resolve(&deps).await
        .context("Failed to resolve dependencies")?;

    println!("{} Resolved {} packages",
        style("✓").green(),
        resolved.len()
    );

    // Download packages
    let pb = ProgressBar::new(resolved.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-")
    );

    for (name, version) in &resolved {
        pb.set_message(format!("Installing {name}@{version}"));

        api_client.download_package(name, version).await
            .context(format!("Failed to download {name}@{version}"))?;

        pb.inc(1);
    }
    pb.finish_with_message("Done!");

    // Update lock file
    let lockfile = Lockfile::new(resolved);
    lockfile.save("bdp.lock")?;

    println!("{} Lock file updated", style("✓").green());

    Ok(())
}
```

## API Client

### api_client.rs

```rust
use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageVersion {
    pub version: String,
    pub checksum: String,
    pub download_url: String,
    pub size_bytes: i64,
}

impl ApiClient {
    pub fn new() -> Result<Self> {
        let base_url = std::env::var("BDP_REGISTRY_URL")
            .unwrap_or_else(|_| "https://api.bdp.dev/v1".to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(format!("bdp-cli/{}", env!("CARGO_PKG_VERSION")))
            .build()?;

        Ok(Self { client, base_url })
    }

    /// Get all versions of a package
    pub async fn get_versions(&self, name: &str) -> Result<Vec<PackageVersion>> {
        let url = format!("{}/packages/{}/versions", self.base_url, name);

        let response = self.client.get(&url)
            .send()
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            anyhow::bail!("Package '{}' not found", name);
        }

        let api_resp: ApiResponse<Vec<PackageVersion>> = response
            .error_for_status()?
            .json()
            .await?;

        Ok(api_resp.data)
    }

    /// Download package tarball
    pub async fn download_package(&self, name: &str, version: &str) -> Result<Vec<u8>> {
        let url = format!("{}/packages/{}/{}/download", self.base_url, name, version);

        let response = self.client.get(&url)
            .send()
            .await
            .context("Failed to download package")?;

        let bytes = response.bytes().await?;

        Ok(bytes.to_vec())
    }

    /// Publish package
    pub async fn publish(&self, tarball: Vec<u8>, token: &str) -> Result<()> {
        let url = format!("{}/packages", self.base_url);

        let response = self.client.post(&url)
            .bearer_auth(token)
            .body(tarball)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Publish failed: {}", error_text);
        }

        Ok(())
    }
}
```

## Dependency Resolution

### resolver.rs

```rust
use anyhow::{Context, Result};
use semver::{Version, VersionReq};
use std::collections::HashMap;
use crate::api_client::ApiClient;

pub struct Resolver {
    client: ApiClient,
}

impl Resolver {
    pub fn new(client: ApiClient) -> Self {
        Self { client }
    }

    /// Resolve dependency graph
    pub async fn resolve(
        &self,
        dependencies: &HashMap<String, String>,
    ) -> Result<HashMap<String, Version>> {
        let mut resolved: HashMap<String, Version> = HashMap::new();
        let mut to_process: Vec<(String, VersionReq)> = vec![];

        // Initialize with direct dependencies
        for (name, req) in dependencies {
            let version_req = VersionReq::parse(req)
                .context(format!("Invalid version requirement: {}", req))?;
            to_process.push((name.clone(), version_req));
        }

        // Process dependency queue
        while let Some((name, req)) = to_process.pop() {
            if let Some(existing) = resolved.get(&name) {
                if !req.matches(existing) {
                    anyhow::bail!(
                        "Version conflict for {}: need {} but have {}",
                        name, req, existing
                    );
                }
                continue;
            }

            // Fetch available versions
            let versions = self.client.get_versions(&name).await?;

            // Find highest compatible version
            let chosen = versions.iter()
                .filter_map(|v| Version::parse(&v.version).ok())
                .filter(|v| req.matches(v))
                .max()
                .context(format!("No compatible version found for {} {}", name, req))?;

            resolved.insert(name.clone(), chosen);

            // TODO: Recursively resolve transitive dependencies
        }

        Ok(resolved)
    }
}
```

## Best Practices

### 1. User-Friendly Output

```rust
use console::{style, Emoji};
use indicatif::{ProgressBar, ProgressStyle};

static SPARKLE: Emoji = Emoji("✨", ":-)");
static ERROR: Emoji = Emoji("❌", "X");

println!("{} {}", SPARKLE, style("Package installed successfully!").green());
println!("{} {}", ERROR, style("Failed to resolve dependency").red());

// Progress bars for long operations
let pb = ProgressBar::new(100);
pb.set_style(
    ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
        .unwrap()
);
```

### 2. Interactive Prompts

```rust
use dialoguer::{Confirm, Select};

// Yes/No confirmation
if Confirm::new()
    .with_prompt("Do you want to continue?")
    .interact()?
{
    // User confirmed
}

// Selection menu
let selection = Select::new()
    .with_prompt("Choose a version")
    .items(&versions)
    .interact()?;
```

### 3. Configuration Management

```rust
// ~/.bdp/config.toml
#[derive(Debug, Deserialize, Serialize)]
pub struct CliConfig {
    pub registry_url: String,
    pub cache_dir: PathBuf,
    pub token: Option<String>,
}

impl CliConfig {
    pub fn load() -> Result<Self> {
        let config_path = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".bdp")
            .join("config.toml");

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)?;
        toml::from_str(&content).context("Invalid config file")
    }
}
```

### 4. Error Handling

```rust
use anyhow::{Context, Result};

pub async fn install_package(name: &str) -> Result<()> {
    let manifest = Manifest::load("bdp.toml")
        .context("Failed to load bdp.toml. Run 'bdp init' first.")?;

    let package = api_client.get_package(name).await
        .context(format!("Failed to fetch package '{}'", name))?;

    // More operations...

    Ok(())
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_parsing() {
        let toml = r#"
            [package]
            name = "test-pkg"
            version = "1.0.0"

            [dependencies]
            foo = "^1.0"
        "#;

        let manifest: Manifest = toml::from_str(toml).unwrap();
        assert_eq!(manifest.package.name, "test-pkg");
        assert_eq!(manifest.dependencies.get("foo"), Some(&"^1.0".to_string()));
    }
}
```

## Resources

- [clap Documentation](https://docs.rs/clap/)
- [dialoguer Documentation](https://docs.rs/dialoguer/)
- [indicatif Documentation](https://docs.rs/indicatif/)
- [semver Documentation](https://docs.rs/semver/)

---

**Next**: See [Next.js Frontend](./nextjs-frontend.md) for web interface.
