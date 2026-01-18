# Just Command Runner Guide

This guide explains how BDP uses Just as a command runner to streamline development workflows and replace traditional shell scripts.

## Table of Contents

- [What is Just?](#what-is-just)
- [Why We Use Just](#why-we-use-just)
- [Installing Just](#installing-just)
- [Getting Started](#getting-started)
- [Common Commands](#common-commands)
- [Command Categories](#command-categories)
- [How to Add New Commands](#how-to-add-new-commands)
- [Comparison with Make and Shell Scripts](#comparison-with-make-and-shell-scripts)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## What is Just?

[Just](https://just.systems) is a command runner and task automation tool, similar to `make` but specifically designed for running commands, not building software. It provides a simple, cross-platform way to save and run project-specific commands.

### Key Features

- **Simple syntax**: Easy-to-read command definitions
- **Cross-platform**: Works on Linux, macOS, and Windows
- **No build artifacts**: Unlike `make`, it doesn't track file dependencies
- **Command runner first**: Designed for running tasks, not building projects
- **Environment variable support**: Loads `.env` files automatically
- **Tab completion**: Supports shell completions for bash, zsh, fish, and PowerShell

## Why We Use Just

BDP adopted Just to replace scattered shell scripts with a unified command interface. Here's why:

### Problems with Shell Scripts

1. **Platform incompatibility**: Bash scripts don't work natively on Windows
2. **Scattered commands**: Scripts spread across multiple directories
3. **Permission issues**: Scripts need `chmod +x` on Unix systems
4. **Hard to discover**: No easy way to see all available commands
5. **Inconsistent patterns**: Each script might use different conventions

### Benefits of Just

1. **Single entry point**: All commands in one `justfile`
2. **Self-documenting**: Run `just --list` to see all commands
3. **Cross-platform**: Same commands work everywhere
4. **No permission issues**: No need to make files executable
5. **Consistent interface**: All commands follow the same pattern
6. **Environment awareness**: Automatically loads `.env` files

### Example Comparison

**Before (Shell Script)**:
```bash
# Multiple steps, platform-specific
./scripts/dev/start-db.sh
./scripts/dev/run-migrations.sh
./scripts/dev/seed-data.sh
cargo run
```

**After (Just)**:
```bash
# Single command, cross-platform
just dev
```

## Installing Just

### Using Cargo (Recommended)

If you have Rust installed:

```bash
cargo install just
```

### Using Package Managers

**macOS (Homebrew)**:
```bash
brew install just
```

**Linux (Various)**:
```bash
# Arch Linux
pacman -S just

# Ubuntu/Debian (requires adding PPA)
# See https://github.com/casey/just#packages

# Nix
nix-env -i just
```

**Windows**:
```powershell
# Using Scoop
scoop install just

# Using Chocolatey
choco install just

# Using Cargo
cargo install just
```

### Verify Installation

```bash
just --version
```

You should see output like: `just 1.25.2`

### Shell Completions (Optional)

Enable tab completion for your shell:

**Bash**:
```bash
# Add to ~/.bashrc
source <(just --completions bash)
```

**Zsh**:
```bash
# Add to ~/.zshrc
source <(just --completions zsh)
```

**Fish**:
```bash
# Add to ~/.config/fish/config.fish
just --completions fish | source
```

**PowerShell**:
```powershell
# Add to your PowerShell profile
just --completions powershell | Out-String | Invoke-Expression
```

## Getting Started

### View All Commands

The first thing to do after installing Just is see what commands are available:

```bash
just --list
# or simply
just
```

This displays all available commands with their descriptions.

### Running a Command

To run a command, simply type `just` followed by the command name:

```bash
just setup      # Run first-time setup
just dev        # Start development server
just test       # Run tests
```

### Commands with Arguments

Some commands accept arguments:

```bash
just db-migrate-add create_users_table    # Add new migration
just test-one test_create_organization    # Run specific test
```

### Viewing Command Details

To see what a command does without running it, use `--show`:

```bash
just --show dev
```

This displays the command definition from the justfile.

## Common Commands

Here are the most frequently used commands in BDP:

### Setup and Installation

```bash
just setup           # Complete first-time setup
just install-deps    # Install all dependencies
just env-setup       # Create .env file
just verify          # Verify setup is correct
```

### Database Management

```bash
just db-up           # Start database
just db-down         # Stop database
just db-migrate      # Run migrations
just db-migrate-add NAME  # Create new migration
just db-shell        # Access database shell
just db-logs         # View database logs
just db-reset        # Reset database (WARNING: destructive)
```

### Development

```bash
just dev             # Start backend server
just web             # Start frontend server
just dev-all         # Start all services
just watch           # Watch and rebuild on changes
```

### Testing

```bash
just test            # Run all tests
just test-unit       # Run unit tests
just test-integration # Run integration tests
just test-verbose    # Run tests with output
just test-one TEST   # Run specific test
```

### Code Quality

```bash
just fmt             # Format code
just lint            # Run linters
just fix             # Auto-fix linting issues
just ci              # Run all CI checks
```

### Building

```bash
just build           # Build backend
just build-release   # Build optimized release
just build-web       # Build frontend
just build-all       # Build everything
```

### SQLx Management

```bash
just sqlx-prepare    # Generate SQLx metadata
just sqlx-check      # Verify metadata is current
just sqlx-clean      # Clean metadata files
```

### Utilities

```bash
just info            # Show environment info
just check-db        # Check database connection
just health          # Check service health
just clean           # Clean build artifacts
```

## Command Categories

The BDP justfile organizes commands into logical categories:

### 1. Setup & Installation
Commands for first-time setup and dependency management.

### 2. Database Management
Commands for working with PostgreSQL databases.

### 3. SQLx Management
Commands for managing SQLx offline metadata.

### 4. Development
Commands for running development servers and tools.

### 5. Testing
Commands for running various test suites.

### 6. Building
Commands for building the project in different configurations.

### 7. CI/CD Simulation
Commands for running CI checks locally.

### 8. Cleanup
Commands for cleaning build artifacts and resetting state.

### 9. MinIO / S3
Commands for managing MinIO object storage.

### 10. Data Ingestion
Commands for running data ingestion pipelines.

### 11. CLI Tool
Commands for building and running the BDP CLI.

### 12. Utilities
Miscellaneous helper commands.

## How to Add New Commands

### Basic Command

To add a new command, edit the `justfile` in the project root:

```just
# Description of what this command does
command-name:
    @echo "Running command..."
    cargo build
    @echo "✓ Done!"
```

**Key points:**
- Use `kebab-case` for command names
- Add a comment above describing what it does
- Use `@echo` to provide user feedback
- The `@` prefix suppresses command echoing

### Command with Parameters

```just
# Build a specific crate
build-crate CRATE:
    @echo "Building {{CRATE}}..."
    cargo build -p {{CRATE}}
```

### Command with Dependencies

Run other commands before this one:

```just
# Setup test database (depends on db-up)
db-test-setup: db-up
    @echo "Applying test migrations..."
    sqlx migrate run --database-url ${TEST_DATABASE_URL}
```

### Command with Default Values

```just
# Run tests with optional filter
test-filter PATTERN="*":
    cargo test {{PATTERN}}
```

### Multi-line Commands

Use shell script syntax for complex commands:

```just
# Complex setup with error handling
complex-setup:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -f .env ]; then
        cp .env.example .env
        echo "✓ Created .env"
    else
        echo "⚠ .env already exists"
    fi
```

### Private Commands

Use `_` prefix for internal commands:

```just
# Internal helper (not shown in --list)
_ensure-database:
    @docker compose ps postgres | grep -q "Up" || just db-up
```

### Environment Variables

Access environment variables with `$VAR` or `${VAR}`:

```just
# Connect to database
db-connect:
    psql ${DATABASE_URL}
```

## Comparison with Make and Shell Scripts

### Just vs Make

| Feature | Just | Make |
|---------|------|------|
| **Purpose** | Command runner | Build system |
| **Dependency tracking** | No | Yes (file-based) |
| **Cross-platform** | Yes | Limited (GNU Make) |
| **Syntax** | Simple, intuitive | Complex, arcane |
| **Tab character** | Not required | Required (error-prone) |
| **Command echoing** | Optional (`@` prefix) | On by default |
| **Environment files** | Auto-loads `.env` | Manual |
| **Use case** | Running tasks | Building software |

**When to use Make**: Building compiled software with complex dependencies.

**When to use Just**: Running project commands, development tasks, CI/CD workflows.

### Just vs Shell Scripts

| Feature | Just | Shell Scripts |
|---------|------|---------------|
| **Discoverability** | `just --list` | Need documentation |
| **Cross-platform** | Yes | No (bash on Windows requires WSL/Git Bash) |
| **Organization** | Single file | Multiple files |
| **Permissions** | Not needed | Need `chmod +x` |
| **Documentation** | Built-in (comments) | Separate docs |
| **Consistency** | Enforced by tool | Manual |

**When to use Shell Scripts**: Complex scripts with many conditionals, loops, or when Just isn't available.

**When to use Just**: Most development tasks, simple workflows, command shortcuts.

### Migration Example

**Before (scripts/dev/start.sh)**:
```bash
#!/bin/bash
set -e

echo "Starting services..."
docker compose up -d postgres
sleep 3
sqlx migrate run
cargo run --bin bdp-server
```

**After (justfile)**:
```just
# Start development server
dev: db-up
    @echo "🚀 Starting backend server..."
    cargo run --bin bdp-server
```

Benefits:
- Shorter and clearer
- Reuses existing `db-up` command
- Cross-platform
- No permission issues
- Self-documenting

## Best Practices

### 1. Use Descriptive Names

**Good**:
```just
db-migrate-add NAME:
    sqlx migrate add {{NAME}}
```

**Bad**:
```just
mig NAME:
    sqlx migrate add {{NAME}}
```

### 2. Add Clear Descriptions

```just
# Generate SQLx offline metadata for CI builds
sqlx-prepare:
    cargo sqlx prepare --workspace -- --all-targets
```

### 3. Provide User Feedback

```just
db-up:
    @echo "🐘 Starting PostgreSQL..."
    docker compose up -d postgres
    @echo "⏳ Waiting for database..."
    @sleep 3
    @echo "✓ Database ready"
```

### 4. Use Icons for Visual Clarity

- 🚀 Starting/launching
- ✓ Success
- ⚠ Warning
- ✗ Error
- 🧪 Testing
- 🔍 Checking/verifying
- 📦 Building/packaging
- 🐘 PostgreSQL
- 🔧 Configuration

### 5. Group Related Commands

Use comments to create sections:

```just
# ============================================================================
# Database Management
# ============================================================================

db-up:
    # ...

db-down:
    # ...
```

### 6. Handle Errors Gracefully

```just
db-reset:
    @echo "⚠️  WARNING: This will delete all data!"
    @echo "Press Ctrl+C to cancel, Enter to continue..."
    @read confirm
    just db-down
    docker compose down postgres -v
    just db-setup
```

### 7. Reuse Commands

```just
# Build then run tests
test-all: build test
```

### 8. Set Default Recipe

```just
# Default recipe - show available commands
default:
    @just --list
```

### 9. Use Appropriate Shell

For simple commands, use the default:
```just
build:
    cargo build
```

For complex commands, use explicit shell:
```just
setup:
    #!/usr/bin/env bash
    set -euo pipefail
    # Complex logic here
```

### 10. Document Parameters

```just
# Create a new database migration
# NAME: descriptive name for the migration (e.g., "add_users_table")
db-migrate-add NAME:
    sqlx migrate add {{NAME}}
```

## Troubleshooting

### Command Not Found

**Problem**: `just: command not found`

**Solution**:
```bash
# Install Just
cargo install just

# Or use package manager
brew install just  # macOS
```

### Command Fails Silently

**Problem**: Command runs but doesn't show output

**Solution**: Remove `@` prefix to see all commands:
```just
# Before (silent)
build:
    @cargo build

# After (verbose)
build:
    cargo build
```

### Environment Variables Not Working

**Problem**: `$DATABASE_URL` is empty

**Solution**:
1. Ensure `.env` file exists:
   ```bash
   just env-setup
   ```

2. Check if variable is set:
   ```bash
   cat .env | grep DATABASE_URL
   ```

3. Just automatically loads `.env` in the same directory as the justfile.

### Command Not Appearing in List

**Problem**: Command exists but doesn't show in `just --list`

**Solution**:
- Commands starting with `_` are private
- Add a comment above the command:
  ```just
  # This command will now appear
  my-command:
      echo "Hello"
  ```

### Cross-Platform Issues

**Problem**: Command works on Linux/macOS but fails on Windows

**Solution**: Use cross-platform tools or check platform:
```just
clean:
    #!/usr/bin/env bash
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
        rm -rf target
    else
        rm -rf target
    fi
```

Or use cross-platform commands:
```just
clean:
    cargo clean  # Works everywhere
```

### Tab Completion Not Working

**Problem**: Tab completion doesn't show Just commands

**Solution**: Enable completions for your shell (see [Shell Completions](#shell-completions-optional)).

## Advanced Features

### Conditional Execution

```just
# Only run if file doesn't exist
init:
    [ -f .env ] || cp .env.example .env
```

### Running in Background

```just
# Start service in background
dev-bg:
    cargo run --bin bdp-server &
```

### Using Specific Shell

```just
# Use bash specifically
bash-command:
    #!/usr/bin/env bash
    echo "Running in bash"

# Use Python
python-command:
    #!/usr/bin/env python3
    print("Running Python!")
```

### Set Variables

```just
# Set variables for all recipes
export DATABASE_URL := "postgresql://localhost/bdp"

db-connect:
    psql $DATABASE_URL
```

### Command Aliases

```just
alias t := test
alias b := build
alias d := dev
```

Now you can use:
```bash
just t   # same as: just test
just b   # same as: just build
just d   # same as: just dev
```

## Resources

- [Just Documentation](https://just.systems)
- [Just GitHub Repository](https://github.com/casey/just)
- [Just Discussions](https://github.com/casey/just/discussions)
- [Justfile Cheat Sheet](https://cheatography.com/linux-china/cheat-sheets/justfile/)

## Next Steps

1. **Explore existing commands**: Run `just --list` to see all available commands
2. **Read the justfile**: Open `justfile` in the project root to see how commands are defined
3. **Try commands**: Start with `just verify` and `just dev`
4. **Add your own**: Follow the patterns in this guide to add new commands
5. **Share improvements**: Submit PRs to improve the justfile

## Summary

Just is a powerful, cross-platform command runner that provides:

- **Unified interface**: All commands in one place
- **Self-documenting**: Easy to discover and understand
- **Cross-platform**: Works on Linux, macOS, and Windows
- **Simple syntax**: Easy to read and write
- **Flexible**: Supports complex workflows and dependencies

By using Just, BDP provides a consistent, discoverable, and maintainable development experience for all contributors.
