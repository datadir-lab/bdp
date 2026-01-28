# CLI Documentation Generation

This document describes the automated CLI documentation generation system for BDP.

## Overview

BDP uses `clap-markdown` to automatically generate CLI reference documentation from the source code. This ensures the documentation always stays in sync with the actual CLI implementation.

## Features

- **Auto-generated from source code** - Documentation is generated directly from Clap derive macros
- **MDX format** - Output is ready for the Next.js documentation site
- **Version controlled** - Generated docs are committed to git for change tracking
- **CI integration** - Can verify docs are up-to-date in CI/CD pipeline

## Architecture

### Components

1. **clap-markdown** - Library that generates markdown from Clap command definitions
2. **xtask** - Cargo task runner for documentation generation
3. **Hidden flag** - `--markdown-help` flag in the CLI for direct generation
4. **Just commands** - Convenient commands to generate docs

### File Locations

- **Source CLI definitions**: `crates/bdp-cli/src/lib.rs`
- **xtask generator**: `xtask/src/main.rs`
- **Generated documentation**: `web/app/[locale]/docs/content/en/cli-reference.mdx`

## Usage

### Generate Documentation

There are three ways to generate CLI documentation:

#### Method 1: Using Just (Recommended)

```bash
# Generate CLI reference documentation
just docs-cli
```

This is the recommended method and runs the xtask to generate the full MDX documentation with frontmatter and examples.

#### Method 2: Direct xtask Invocation

```bash
# Run xtask directly
cargo run --package xtask -- generate-cli-docs

# Generate to a custom directory
cargo run --package xtask -- generate-cli-docs --output-dir path/to/output
```

#### Method 3: Using the Hidden Flag

```bash
# Generate raw markdown using the CLI itself
cargo run --bin bdp -- --markdown-help > output.md
```

### Check if Documentation is Up-to-Date

For CI/CD pipelines, verify that the documentation is current:

```bash
just docs-cli-check
```

This command will:
- Generate fresh documentation
- Compare it with the committed version
- Exit with an error if they differ

## Workflow

### Development Workflow

When you modify CLI commands or arguments:

1. **Make your changes** to `crates/bdp-cli/src/lib.rs`
2. **Generate updated docs**: `just docs-cli`
3. **Review the changes**: Check the diff in `cli-reference.mdx`
4. **Commit both**: Include both code and documentation changes in your commit

### Example

```bash
# Edit CLI command structure
code crates/bdp-cli/src/lib.rs

# Regenerate documentation
just docs-cli

# Review changes
git diff web/app/[locale]/docs/content/en/cli-reference.mdx

# Commit everything together
git add crates/bdp-cli/src/lib.rs web/app/[locale]/docs/content/en/cli-reference.mdx
git commit -m "feat: add --output flag to export command"
```

## Customization

### Modifying the Output Format

The MDX output format can be customized by editing `xtask/src/main.rs`:

```rust
fn generate_cli_docs(output_dir: &str) -> anyhow::Result<()> {
    let markdown = clap_markdown::help_markdown::<bdp_cli::Cli>();

    // Customize the MDX content here
    let mdx_content = format!(
        r#"---
title: Your Custom Title
---

{markdown}

## Your Custom Section

Add custom content here...
"#
    );

    // ...
}
```

### Adding Examples

Edit the `mdx_content` template in `xtask/src/main.rs` to add or modify examples.

### Changing Output Location

By default, documentation is generated to `web/app/[locale]/docs/content/en/cli-reference.mdx`.

To change this:

```bash
cargo run --package xtask -- generate-cli-docs --output-dir custom/path
```

## CI/CD Integration

The CLI documentation generation is fully integrated into the CI/CD pipeline:

### Automatic Generation in Build Workflows

**Web Build** (`just web-build`, `just web-prod`):
- Automatically generates CLI docs before building the frontend
- Ensures the web documentation always includes the latest CLI reference

**Production Build** (`just prod-build`):
- Generates CLI docs as the first step
- Included in all production deployments

**CI Local Check** (`just ci`):
- Includes `docs-cli-check` to verify docs are current

### GitHub Actions Integration

#### CI Workflow (`.github/workflows/ci.yml`)

A dedicated `cli-docs-check` job:
```yaml
- name: Generate CLI docs
  run: cargo run --package xtask -- generate-cli-docs

- name: Check for uncommitted CLI docs changes
  run: |
    if ! git diff --exit-code web/app/\[locale\]/docs/content/en/cli-reference.mdx; then
      echo "❌ CLI documentation is out of date!"
      echo "Please run 'just docs-cli' and commit the changes."
      exit 1
    fi
```

This ensures that:
- CLI docs are always in sync with the code
- PRs will fail if docs are outdated
- Contributors are reminded to update docs

#### Web CI Workflow (`.github/workflows/ci-web.yml`)

Before building the web frontend:
```yaml
- name: Generate CLI documentation
  run: cargo run --package xtask -- generate-cli-docs

- name: Build Next.js site
  run: yarn build
```

This ensures:
- The web build always has the latest CLI docs
- No manual intervention needed for doc updates

### Pre-commit Hook

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash
# Auto-generate CLI docs on commit

if git diff --cached --name-only | grep -q "crates/bdp-cli/src"; then
    echo "🔄 CLI changes detected, regenerating documentation..."
    just docs-cli
    git add web/app/[locale]/docs/content/en/cli-reference.mdx
fi
```

Make it executable:

```bash
chmod +x .git/hooks/pre-commit
```

## Best Practices

1. **Always commit generated docs** - Treat them as part of the source code
2. **Review generated docs** - Check that changes are correct before committing
3. **Update examples** - Keep the examples section in xtask up-to-date
4. **Run in CI** - Use `docs-cli-check` to catch outdated documentation
5. **Document complex flags** - Add detailed help text in the Clap derives

## Troubleshooting

### Documentation is out of sync

**Problem**: The generated documentation doesn't match the CLI

**Solution**:
```bash
# Clean build and regenerate
cargo clean
just docs-cli
```

### Build errors in xtask

**Problem**: `cargo run --package xtask` fails to compile

**Solution**:
```bash
# Check that bdp-cli compiles first
cargo check --package bdp-cli

# Then try xtask again
cargo run --package xtask -- generate-cli-docs
```

### Changes not appearing

**Problem**: Modified CLI help text doesn't show in generated docs

**Solution**:
1. Ensure changes are saved in `crates/bdp-cli/src/lib.rs`
2. Run `cargo clean` to clear cache
3. Regenerate: `just docs-cli`

## Technical Details

### How It Works

1. **Clap Derive Macros** - CLI structure is defined using `#[derive(Parser)]`
2. **clap-markdown** - Traverses the Clap command tree and generates markdown
3. **xtask** - Wraps the generation with MDX frontmatter and examples
4. **File Output** - Writes the complete MDX file to the web docs directory

### Dependencies

- `clap = "4.5"` - CLI framework
- `clap-markdown = "0.1"` - Markdown generation
- `chrono = "0.4"` - Timestamps in generated docs

### Output Format

The generated documentation includes:

- **Frontmatter** - Title and description for Next.js
- **Overview** - Project description and installation instructions
- **Quick Start** - Getting started examples
- **Commands** - Auto-generated command reference from Clap
- **Environment Variables** - Configuration options
- **Examples** - Practical usage examples
- **Support** - Links to help resources

## Future Enhancements

Potential improvements to the documentation generation:

- [ ] Generate per-command pages (one MDX file per command)
- [ ] Add command usage statistics/examples from real usage
- [ ] Generate shell completion scripts
- [ ] Create interactive command builder
- [ ] Multi-language documentation support
- [ ] Generate man pages for Unix systems

## Related Documentation

- [CLI Development Guide](./agents/cli-development.md)
- [Testing Strategy](./agents/testing.md)
- [Next.js Frontend](./agents/nextjs-frontend.md)

## References

- [clap-markdown on crates.io](https://crates.io/crates/clap-markdown)
- [Rust CLI Book - Rendering Documentation](https://rust-cli.github.io/book/in-depth/docs.html)
- [cargo-xtask pattern](https://github.com/matklad/cargo-xtask)
