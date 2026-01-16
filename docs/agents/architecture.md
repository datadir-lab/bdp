# Architecture Overview

## System Design

BDP follows a three-tier architecture optimized for single-server deployment:

```
┌─────────────────────────────────────────────────────┐
│              Users & Researchers                     │
└──────────┬────────────────────────┬─────────────────┘
           │                        │
    CLI Tool (Rust)         Web Browser
           │                        │
           │                  Next.js Frontend
           │                   (port 3000)
           │                        │
           └────────────┬───────────┘
                        │
                   Rust API Server
                   (axum, port 8000)
                        │
            ┌───────────┼───────────┐
            │           │           │
       PostgreSQL   Object      Background
       (port 5432)  Storage       Jobs
```

## Core Components

### 1. Rust API Server (bdp-server)
- **Framework**: axum 0.7
- **Purpose**: RESTful API for package management
- **Responsibilities**:
  - Package CRUD operations
  - Version management
  - Search and discovery
  - Authentication/Authorization
  - Checksum verification
  - Background job orchestration

### 2. CLI Tool (bdp)
- **Framework**: clap 4.x
- **Purpose**: Local package and environment management
- **Key Commands**:
  ```bash
  bdp init              # Initialize new project
  bdp install <pkg>     # Install package
  bdp lock              # Generate lock file
  bdp publish           # Publish to registry
  bdp env create        # Create environment snapshot
  bdp env restore       # Restore from snapshot
  ```

### 3. Web Frontend (bdp-web)
- **Framework**: Next.js 15+ with Nextra
- **Purpose**: Package discovery, documentation, user management
- **Key Features**:
  - Package search and browsing
  - Version comparison
  - Documentation (Nextra MDX)
  - User dashboard
  - Environment visualization

## Database Schema

### Core Tables

```sql
-- Package registry
CREATE TABLE packages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,
    description TEXT,
    repository_url TEXT,
    homepage_url TEXT,
    license VARCHAR(100),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    downloads_total BIGINT DEFAULT 0
);

-- Package versions
CREATE TABLE versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    package_id UUID REFERENCES packages(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    checksum VARCHAR(64) NOT NULL,  -- sha256
    size_bytes BIGINT NOT NULL,
    download_url TEXT NOT NULL,
    published_at TIMESTAMPTZ DEFAULT NOW(),
    yanked BOOLEAN DEFAULT FALSE,
    yanked_reason TEXT,
    metadata JSONB,  -- Flexible for bioinformatics-specific data
    UNIQUE(package_id, version)
);

-- Dependencies between versions
CREATE TABLE dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id UUID REFERENCES versions(id) ON DELETE CASCADE,
    depends_on_package VARCHAR(255) NOT NULL,
    version_requirement VARCHAR(100) NOT NULL,  -- Semver range: ^1.0, >=2.0
    optional BOOLEAN DEFAULT FALSE,
    features JSONB  -- Optional features required
);

-- Saved environments (lock files)
CREATE TABLE environments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID,  -- NULL for anonymous
    name VARCHAR(255) NOT NULL,
    description TEXT,
    lock_file JSONB NOT NULL,  -- Complete resolved dependency graph
    created_at TIMESTAMPTZ DEFAULT NOW(),
    forked_from UUID REFERENCES environments(id)
);

-- Users
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(100) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    api_token_hash VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    is_active BOOLEAN DEFAULT TRUE
);

-- Full-text search index
CREATE INDEX packages_search_idx ON packages
    USING GIN (to_tsvector('english', name || ' ' || COALESCE(description, '')));

-- Performance indexes
CREATE INDEX versions_package_id_idx ON versions(package_id);
CREATE INDEX dependencies_version_id_idx ON dependencies(version_id);
CREATE INDEX environments_user_id_idx ON environments(user_id);
```

## Package Format

### Manifest File: `bdp.toml`

```toml
[package]
name = "samtools"
version = "1.18.0"
description = "Tools for manipulating SAM/BAM/CRAM files"
authors = ["Bioinformatics Team <team@example.org>"]
license = "MIT"
repository = "https://github.com/samtools/samtools"
homepage = "http://www.htslib.org/"
documentation = "http://www.htslib.org/doc/samtools.html"
keywords = ["bioinformatics", "genomics", "alignment"]

[dependencies]
htslib = "^1.18"
ncurses = ">=6.0"

[optional-dependencies]
plotting = ["r-ggplot2 >= 3.0"]

[build]
type = "make"  # Options: make, cmake, cargo, pip, conda
commands = [
    "autoheader",
    "autoconf -Wno-syntax",
    "./configure --prefix=${BDP_INSTALL_PREFIX}",
    "make",
    "make install"
]

[metadata]
# Bioinformatics-specific metadata
bioconda_compatible = true
container_image = "quay.io/biocontainers/samtools:1.18"
citation = "doi:10.1093/bioinformatics/btp352"
```

### Lock File: `bdp.lock`

```json
{
  "version": 1,
  "resolved_at": "2026-01-16T12:34:56Z",
  "packages": {
    "samtools": {
      "version": "1.18.0",
      "checksum": "sha256:abc123...",
      "url": "https://registry.bdp.dev/packages/samtools-1.18.0.tar.gz",
      "size": 4294967296
    },
    "htslib": {
      "version": "1.18.1",
      "checksum": "sha256:def456...",
      "url": "https://registry.bdp.dev/packages/htslib-1.18.1.tar.gz",
      "size": 2147483648
    },
    "ncurses": {
      "version": "6.4.0",
      "checksum": "sha256:789abc...",
      "url": "https://registry.bdp.dev/packages/ncurses-6.4.0.tar.gz",
      "size": 3355443200
    }
  },
  "checksums": {
    "bdp.toml": "sha256:checksumofmanifest..."
  }
}
```

## API Design

### REST API Endpoints

```
Base URL: https://api.bdp.dev/v1

# Package Management
GET     /packages                    # List all packages (paginated)
GET     /packages/:name              # Get package metadata
POST    /packages                    # Publish new package (auth)
PATCH   /packages/:name              # Update package metadata (auth)

# Versions
GET     /packages/:name/versions     # List all versions
GET     /packages/:name/versions/:v  # Get specific version
POST    /packages/:name/versions     # Publish new version (auth)
DELETE  /packages/:name/versions/:v  # Yank version (auth)

# Downloads
GET     /packages/:name/:version/download  # Download package tarball

# Search
GET     /search?q=alignment&limit=20  # Search packages

# Dependencies
GET     /packages/:name/versions/:v/dependencies  # Get deps tree

# Environments
GET     /environments                # List saved environments
POST    /environments                # Save environment
GET     /environments/:id            # Get environment details
POST    /environments/:id/fork       # Fork environment

# Authentication
POST    /auth/register               # User registration
POST    /auth/login                  # User login
POST    /auth/token                  # Generate API token
DELETE  /auth/token                  # Revoke token
```

### API Response Format

```json
{
  "success": true,
  "data": {
    "package": {
      "name": "samtools",
      "description": "...",
      "versions": ["1.18.0", "1.17.0"]
    }
  },
  "meta": {
    "pagination": {
      "page": 1,
      "per_page": 20,
      "total": 150
    }
  }
}
```

Error response:
```json
{
  "success": false,
  "error": {
    "code": "PACKAGE_NOT_FOUND",
    "message": "Package 'nonexistent' not found",
    "details": {}
  }
}
```

## Dependency Resolution

### Algorithm: PubGrub (like Cargo)

1. Start with direct dependencies from `bdp.toml`
2. For each dependency:
   - Parse version requirement (semver)
   - Query registry for compatible versions
   - Select highest compatible version
3. Recursively resolve transitive dependencies
4. Check for conflicts
5. If conflict: backtrack and try alternative versions
6. Generate `bdp.lock` with resolved graph

### Conflict Resolution

```rust
// Example conflict scenario
samtools -> htslib ^1.18
blast    -> htslib ^1.16

// Resolution: Find highest version satisfying both
// Result: htslib 1.18.x (compatible with ^1.16 and ^1.18)
```

## Security Considerations

1. **Checksum Verification**: All packages verified with SHA-256
2. **HTTPS Only**: All API communication over TLS
3. **API Token Authentication**: JWT tokens for publishing
4. **Rate Limiting**: Prevent abuse (tower-governor)
5. **Input Validation**: Strict validation of package names, versions
6. **Package Signing**: (Future) GPG signatures for packages

## Data Flow Examples

### Publishing a Package

```
1. User: bdp publish
2. CLI: Read bdp.toml, create tarball
3. CLI: POST /packages with tarball + metadata
4. Server: Validate manifest
5. Server: Compute checksum
6. Server: Upload to object storage
7. Server: Insert into database
8. Server: Return success
9. CLI: Display confirmation
```

### Installing a Package

```
1. User: bdp install samtools
2. CLI: Read bdp.toml (if exists)
3. CLI: GET /packages/samtools/versions
4. CLI: Resolve dependencies locally
5. CLI: For each package:
   - GET /packages/:name/:version/download
   - Verify checksum
   - Extract to cache
6. CLI: Write bdp.lock
7. CLI: Execute build commands (if needed)
8. CLI: Display success
```

## Scalability Considerations

Even with single-server deployment:

- **Connection Pooling**: PostgreSQL connection pool (sqlx)
- **Caching**: Redis or in-memory cache for hot packages
- **CDN**: CloudFlare/similar for package downloads
- **Indexes**: Proper database indexes for queries
- **Background Jobs**: Async job processing (apalis)

## Design Principles

1. **Simplicity First**: Prefer simple solutions
2. **Explicit Over Implicit**: Clear error messages
3. **Fail Fast**: Validate early, fail loudly
4. **Reproducibility**: Lock files ensure identical environments
5. **Community-Driven**: Easy package publishing
6. **Standards-Based**: Follow semver, use standard formats

---

**Next**: See [Rust Backend](./rust-backend.md) for implementation details.
