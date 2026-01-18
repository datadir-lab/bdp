# BDP CLI Commands

This directory contains all command implementations for the BDP CLI tool.

## Structure

Each command is typically implemented as a separate module with:

- Command definition and argument parsing
- Business logic implementation
- Output formatting
- Error handling

## Available Commands

Commands are organized by functionality:

- **Query commands** - Search and retrieve biological data
- **Data management** - Import, export, and validation
- **Administrative** - Server management and configuration
- **Utility** - Helper commands and tools

## Adding New Commands

1. Create a new module file (e.g., `my_command.rs`)
2. Define command structure using `clap`:
   ```rust
   use clap::Args;

   #[derive(Debug, Args)]
   pub struct MyCommand {
       /// Description of the argument
       #[arg(short, long)]
       pub option: String,
   }
   ```
3. Implement the command logic
4. Register the command in `mod.rs`
5. Add tests and documentation

## Example Command

```rust
use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct SearchCommand {
    /// Query string to search for
    pub query: String,

    /// Maximum number of results
    #[arg(short, long, default_value = "10")]
    pub limit: usize,
}

impl SearchCommand {
    pub async fn execute(&self) -> Result<()> {
        // Implementation here
        Ok(())
    }
}
```

## Guidelines

- Use `clap` for argument parsing
- Return `anyhow::Result` for error handling
- Format output clearly (consider using `prettytable` or similar)
- Add `--json` flag for machine-readable output
- Include examples in help text
