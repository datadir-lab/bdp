# BDP Configuration

Configuration files for different deployment environments.

## Directory Structure

```
config/
├── development/    # Development environment configuration
├── production/     # Production environment configuration
└── README.md       # This file
```

## Configuration Files

Configuration is managed through:

1. **Environment variables** - For sensitive data and runtime configuration
2. **TOML files** - For static configuration in this directory
3. **`.env` files** - For local development (not committed to git)

## Development Configuration (`development/`)

Configuration for local development:

- Local database connections
- Debug logging enabled
- Development-friendly timeouts
- Test API keys
- Mock external services

Example files:
- `server.toml` - Server configuration
- `database.toml` - Database connection settings
- `ingestion.toml` - Data ingestion settings

## Production Configuration (`production/`)

Configuration for production deployment:

- Production database connections
- Optimized logging levels
- Production timeouts and limits
- Rate limiting configuration
- Security settings

## Configuration Priority

Configuration values are loaded in this order (later overrides earlier):

1. Default values in code
2. Configuration files in `config/{environment}/`
3. Environment variables
4. Command-line arguments

## Environment Variables

Key environment variables:

```bash
# Application
BDP_ENV=development|production
BDP_LOG_LEVEL=debug|info|warn|error

# Database
DATABASE_URL=postgresql://user:pass@host:port/dbname
DATABASE_POOL_SIZE=10

# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# API Keys (production only)
NCBI_API_KEY=your-key-here
UNIPROT_API_KEY=your-key-here
```

## Security

**IMPORTANT:**

- Never commit sensitive data (API keys, passwords) to git
- Use `.env` files for local secrets (already in `.gitignore`)
- Use secrets management systems in production (e.g., AWS Secrets Manager)
- Rotate API keys regularly
- Use different keys for development and production

## Adding New Configuration

1. Add default values in the relevant Rust struct
2. Add configuration file in `config/{environment}/`
3. Document the new configuration in this README
4. Update `.env.example` with new environment variables

## Loading Configuration

Configuration is loaded using the `config` crate in Rust:

```rust
use config::{Config, Environment, File};

let config = Config::builder()
    .add_source(File::with_name("config/development/server"))
    .add_source(Environment::with_prefix("BDP"))
    .build()?;
```

## Validation

Configuration is validated at startup. The application will fail to start if:

- Required values are missing
- Values are invalid (e.g., invalid URLs)
- Database connection fails
- Required services are unreachable

## Environment-Specific Files

Create environment-specific configuration files:

```
config/
├── development/
│   ├── server.toml
│   ├── database.toml
│   └── ingestion.toml
└── production/
    ├── server.toml
    ├── database.toml
    └── ingestion.toml
```

Set `BDP_ENV` to load the correct configuration:

```bash
export BDP_ENV=production
cargo run
```
