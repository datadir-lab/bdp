# BDP - Bioinformatics Dependencies Platform
# Just command runner - replaces all shell scripts
# Install: cargo install just
# Usage: just <command>

# Set shell for Windows compatibility
set shell := ["powershell.exe", "-NoLogo", "-Command"]

# Default recipe - show available commands
default:
    @just --list

# ============================================================================
# Setup & Installation
# ============================================================================

# Complete first-time setup (quick start)
setup: install-deps env-setup db-setup db-migrate
    @echo "✓ Setup complete! Run 'just dev' to start development"

# Install all dependencies
install-deps:
    @echo "📦 Installing dependencies..."
    cargo install sqlx-cli --features postgres
    @cd web; yarn install

# Setup environment file
env-setup:
    #!/usr/bin/env bash
    if [ ! -f .env ]; then
        cp .env.example .env
        echo "✓ Created .env from .env.example"
    else
        echo "⚠ .env already exists, skipping"
    fi

# Verify setup is correct
verify:
    @echo "🔍 Verifying setup..."
    @echo "\n📋 Required Files:"
    @test -f .env.example && echo "  ✓ .env.example" || echo "  ✗ .env.example"
    @test -f Cargo.toml && echo "  ✓ Cargo.toml" || echo "  ✗ Cargo.toml"
    @test -f docker-compose.yml && echo "  ✓ docker-compose.yml" || echo "  ✗ docker-compose.yml"
    @echo "\n🐳 Docker:"
    @docker --version > /dev/null 2>&1 && echo "  ✓ Docker installed" || echo "  ✗ Docker not found"
    @docker compose version > /dev/null 2>&1 && echo "  ✓ Docker Compose installed" || echo "  ✗ Docker Compose not found"
    @echo "\n🦀 Rust Toolchain:"
    @rustc --version 2>&1 | head -n1 | sed 's/^/  ✓ /'
    @cargo --version 2>&1 | sed 's/^/  ✓ /'
    @echo "\n⚡ SQLx CLI:"
    @sqlx --version 2>&1 | sed 's/^/  ✓ /' || echo "  ✗ sqlx-cli not installed (run: cargo install sqlx-cli --features postgres)"
    @echo "\n📦 Node.js:"
    @node --version 2>&1 | sed 's/^/  ✓ Node /'
    @npm --version 2>&1 | sed 's/^/  ✓ npm /'
    @echo "\n✓ Verification complete"

# ============================================================================
# Database Management
# ============================================================================

# Start development database
db-up:
    @Write-Host "🐘 Starting PostgreSQL..."
    @docker compose up -d postgres
    @Write-Host "⏳ Waiting for database..."
    @Start-Sleep -Seconds 3
    @Write-Host "✓ Database ready"

# Stop database
db-down:
    @echo "Stopping PostgreSQL..."
    docker compose down postgres

# Start test database
db-test-up:
    @echo "🧪 Starting test database..."
    docker compose up -d postgres-test
    @echo "⏳ Waiting for test database..."
    @sleep 3
    @echo "✓ Test database ready"

# Stop test database
db-test-down:
    @echo "Stopping test database..."
    docker compose down postgres-test

# Complete database setup (start + migrate)
db-setup: db-up
    @sleep 2

# Run database migrations
db-migrate:
    @Write-Host "🔄 Running migrations..."
    @sqlx migrate run
    @Write-Host "✓ Migrations complete"

# Revert last migration
db-migrate-revert:
    @echo "⏪ Reverting last migration..."
    sqlx migrate revert
    @echo "✓ Migration reverted"

# Create new migration
db-migrate-add NAME:
    @echo "📝 Creating migration: {{NAME}}"
    sqlx migrate add {{NAME}}
    @echo "✓ Migration file created in migrations/"

# Reset database (dangerous - drops all data)
db-reset:
    @echo "⚠️  WARNING: This will delete all data!"
    @echo "Press Ctrl+C to cancel, Enter to continue..."
    @read confirm
    docker compose down postgres -v
    @echo "✓ Database reset"
    just db-setup db-migrate

# Seed development data
db-seed:
    @echo "🌱 Seeding database..."
    psql ${DATABASE_URL} -f scripts/seed-data.sql
    @echo "✓ Database seeded"

# Connect to database with psql
db-shell:
    @echo "🐘 Connecting to database..."
    psql ${DATABASE_URL}

# Database logs
db-logs:
    docker compose logs -f postgres

# ============================================================================
# SQLx Management
# ============================================================================

# Generate SQLx offline metadata
sqlx-prepare:
    @echo "📦 Generating SQLx metadata..."
    cargo sqlx prepare --workspace -- --all-targets
    @echo "✓ Metadata generated in .sqlx/"
    @echo "ℹ️  Commit .sqlx/ files to git for offline builds"

# Verify SQLx metadata is up to date
sqlx-check:
    @echo "🔍 Verifying SQLx metadata..."
    cargo sqlx prepare --check --workspace -- --all-targets
    @echo "✓ SQLx metadata is current"

# Clean SQLx metadata
sqlx-clean:
    @echo "🧹 Cleaning SQLx metadata..."
    rm -rf .sqlx
    @echo "✓ SQLx metadata cleaned"

# ============================================================================
# Development
# ============================================================================

# Start development (database + backend server)
dev: db-up
    @Write-Host "🚀 Starting backend server..."
    @cargo run --bin bdp-server

# Start frontend development server (quick dev mode)
web-dev:
    @Write-Host "🌐 Starting frontend (dev mode)..."
    @cd web; yarn dev

# Build frontend with Pagefind indexing and start production server
web:
    @Write-Host "🌐 Building frontend..."
    @cd web; $env:NEXT_PRIVATE_DISABLE_TURBO="1"; yarn build
    @Write-Host "🔍 Indexing documentation with Pagefind..."
    @cd web; yarn pagefind
    @Write-Host "✓ Build complete with Pagefind index"
    @Write-Host "🌐 Starting production server..."
    @cd web; yarn start

# Start all services (backend + frontend + database) in dev mode
dev-all: db-up
    @echo "🚀 Starting all services..."
    @echo "Backend: http://localhost:8000"
    @echo "Frontend: http://localhost:3000"
    @just dev & just web-dev

# Watch and rebuild on changes
watch:
    cargo watch -x 'run --bin bdp-server'

# Format code
fmt:
    @echo "🎨 Formatting code..."
    cargo fmt --all
    @cd web; yarn format
    @echo "✓ Code formatted"

# Lint code
lint:
    @echo "🔍 Linting code..."
    cargo clippy --all-targets --all-features -- -D warnings
    @cd web; yarn lint
    @echo "✓ Linting complete"

# Fix linting issues
fix:
    @echo "🔧 Fixing linting issues..."
    cargo clippy --fix --allow-dirty --allow-staged
    cargo fmt --all
    @echo "✓ Fixes applied"

# ============================================================================
# Building
# ============================================================================

# Build all Rust crates
build:
    @echo "🔨 Building Rust workspace..."
    cargo build --workspace

# Build release version
build-release:
    @echo "🔨 Building release version..."
    cargo build --workspace --release

# Build frontend
build-web:
    @Write-Host "🔨 Building frontend..."
    @cd web; $env:NEXT_PRIVATE_DISABLE_TURBO="1"; yarn build

# Build all (backend + frontend)
build-all: build build-web
    @echo "✓ All builds complete"

# Build Docker images
docker-build:
    @echo "🐳 Building Docker images..."
    docker build -f docker/Dockerfile.server -t bdp-server:latest .
    docker build -f docker/Dockerfile.cli -t bdp-cli:latest .
    docker build -f docker/Dockerfile.ingest -t bdp-ingest:latest .
    docker build -f docker/Dockerfile.web -t bdp-web:latest .
    @echo "✓ Docker images built"

# ============================================================================
# Testing
# ============================================================================

# Run all tests
test: db-test-up
    @echo "🧪 Running tests..."
    TEST_DATABASE_URL="postgresql://bdp:bdp_test_password@localhost:5433/bdp_test" \
    cargo test --workspace --all-features
    @echo "✓ Tests complete"

# Run tests with output
test-verbose: db-test-up
    @echo "🧪 Running tests (verbose)..."
    TEST_DATABASE_URL="postgresql://bdp:bdp_test_password@localhost:5433/bdp_test" \
    cargo test --workspace --all-features -- --nocapture

# Run integration tests only
test-integration: db-test-up
    @echo "🧪 Running integration tests..."
    TEST_DATABASE_URL="postgresql://bdp:bdp_test_password@localhost:5433/bdp_test" \
    cargo test --test '*' --all-features

# Run unit tests only
test-unit:
    @echo "🧪 Running unit tests..."
    cargo test --workspace --lib --all-features

# Run specific test
test-one TEST:
    @echo "🧪 Running test: {{TEST}}"
    cargo test {{TEST}} -- --nocapture

# Test with coverage
test-coverage:
    @echo "🧪 Running tests with coverage..."
    cargo tarpaulin --workspace --all-features --out Html --output-dir coverage

# Reset and run tests
test-fresh: db-test-down db-test-up test
    @echo "✓ Fresh tests complete"

# ============================================================================
# CLI Testing
# ============================================================================

# Set up test directory for CLI testing (IMPORTANT: Always use external directory)
test-cli-setup:
    @echo "📁 Setting up CLI test directory..."
    mkdir -p D:/dev/datadir/bdp-example
    @echo "✓ Test directory ready at D:/dev/datadir/bdp-example"

# Clean CLI test directory
test-cli-clean:
    @echo "🧹 Cleaning CLI test directory..."
    rm -rf D:/dev/datadir/bdp-example/*
    @echo "✓ Test directory cleaned"

# Run CLI command in test directory
test-cli CMD:
    @echo "🔧 Running: bdp {{CMD}}"
    cd D:/dev/datadir/bdp-example && cargo run --bin bdp -- {{CMD}}

# Full CLI test workflow
test-cli-full: test-cli-setup
    @echo "🧪 Running full CLI test workflow..."
    @echo "\n1. Initialize project..."
    cd D:/dev/datadir/bdp-example && cargo run --bin bdp -- init --name test-project
    @echo "\n2. Add sources..."
    cd D:/dev/datadir/bdp-example && cargo run --bin bdp -- source add "uniprot:P01308-fasta@1.0"
    @echo "\n3. List sources..."
    cd D:/dev/datadir/bdp-example && cargo run --bin bdp -- source list
    @echo "\n✓ CLI test workflow complete"

# ============================================================================
# CI/CD Simulation
# ============================================================================

# Run all CI checks locally
ci: sqlx-check lint test
    @echo "✓ All CI checks passed!"

# Run CI checks in offline mode (like GitHub Actions)
ci-offline:
    @echo "🔍 Running CI checks (offline mode)..."
    SQLX_OFFLINE=true cargo check --workspace --all-features
    SQLX_OFFLINE=true cargo clippy --workspace --all-features -- -D warnings
    SQLX_OFFLINE=true cargo test --workspace --lib
    @echo "✓ Offline CI checks passed!"

# ============================================================================
# Cleanup
# ============================================================================

# Clean build artifacts
clean:
    @echo "🧹 Cleaning build artifacts..."
    cargo clean
    @cd web; Remove-Item -Recurse -Force .next, node_modules/.cache -ErrorAction SilentlyContinue
    @echo "✓ Cleaned"

# Deep clean (including dependencies)
clean-all: clean
    @echo "🧹 Deep cleaning..."
    @cd web; Remove-Item -Recurse -Force node_modules -ErrorAction SilentlyContinue
    rm -rf target
    @echo "✓ Deep cleaned"

# Stop all Docker services
stop:
    @echo "🛑 Stopping all services..."
    docker compose down
    @echo "✓ Services stopped"

# Stop all and remove volumes (deletes data)
stop-all:
    @echo "🛑 Stopping all services and removing volumes..."
    docker compose down -v
    @echo "✓ Services stopped, volumes removed"

# ============================================================================
# Docker Compose - Full Stack
# ============================================================================

# Start all services with Docker Compose (DB + Backend + MinIO)
docker-up:
    @Write-Host "🐳 Starting all services..."
    @docker compose up -d
    @Write-Host "⏳ Waiting for services to be ready..."
    @Start-Sleep -Seconds 5
    @Write-Host "✓ Services started"
    @Write-Host "  🗄️  PostgreSQL:   localhost:5432"
    @Write-Host "  🚀 Backend API:   http://localhost:8000"
    @Write-Host "  📦 MinIO Console: http://localhost:9001 (minioadmin/minioadmin)"
    @Write-Host ""
    @Write-Host "💡 Run migrations: just docker-migrate"
    @Write-Host "💡 Start frontend: just web"

# Stop all Docker Compose services
docker-down:
    @Write-Host "🛑 Stopping all services..."
    @docker compose down
    @Write-Host "✓ Services stopped"

# Run migrations in Docker container
docker-migrate:
    @Write-Host "🔄 Running migrations in Docker..."
    @docker compose exec bdp-server sqlx migrate run
    @Write-Host "✓ Migrations complete"

# View logs from all services
docker-logs:
    docker compose logs -f

# View backend logs
docker-logs-backend:
    docker compose logs -f bdp-server

# Restart backend service
docker-restart-backend:
    @echo "🔄 Restarting backend..."
    docker compose restart bdp-server
    @echo "✓ Backend restarted"

# Full stack with migrations (recommended for first time)
docker-setup: docker-up
    @Write-Host "⏳ Waiting for database to be ready..."
    @Start-Sleep -Seconds 3
    @just docker-migrate
    @Write-Host ""
    @Write-Host "✅ Full stack ready!"
    @Write-Host "  🌐 Start frontend: cd web && yarn dev"
    @Write-Host "  🌐 Frontend URL:   http://localhost:3000"

# ============================================================================
# MinIO / S3
# ============================================================================

# Start MinIO
minio-up:
    @echo "📦 Starting MinIO..."
    docker compose up -d minio minio-init
    @echo "✓ MinIO ready at http://localhost:9001"

# Stop MinIO
minio-down:
    docker compose down minio minio-init

# MinIO logs
minio-logs:
    docker compose logs -f minio

# ============================================================================
# Data Ingestion
# ============================================================================

# Run UniProt ingestion
ingest-uniprot:
    @echo "🔬 Starting UniProt ingestion..."
    cargo run --bin bdp-ingest -- uniprot

# Run NCBI ingestion (future)
ingest-ncbi:
    @echo "🔬 Starting NCBI ingestion..."
    cargo run --bin bdp-ingest -- ncbi

# Run all ingestion
ingest-all: ingest-uniprot
    @echo "✓ All ingestion complete"

# ============================================================================
# CLI Tool
# ============================================================================

# Build and install CLI locally
cli-install:
    @echo "📦 Installing bdp CLI..."
    cargo install --path crates/bdp-cli
    @echo "✓ CLI installed (run 'bdp --help')"

# Run CLI (init example)
cli-init:
    cargo run --bin bdp-cli -- init

# Run CLI (help)
cli-help:
    cargo run --bin bdp-cli -- --help

# ============================================================================
# Documentation
# ============================================================================

# Build documentation
docs:
    @echo "📚 Building documentation..."
    cargo doc --workspace --no-deps --open
    @echo "✓ Documentation ready"

# Serve frontend docs
docs-web:
    @echo "📚 Starting documentation server..."
    cd web && yarn dev

# ============================================================================
# Deployment
# ============================================================================

# Build for production
prod-build: build-release build-web docker-build
    @echo "✓ Production build complete"

# Deploy to production (placeholder)
deploy:
    @echo "🚀 Deploying to production..."
    @echo "⚠️  Deploy script not implemented yet"

# ============================================================================
# Utilities
# ============================================================================

# Show environment info
info:
    @echo "📊 BDP Environment Info"
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    @echo "Rust:        $(rustc --version)"
    @echo "Cargo:       $(cargo --version)"
    @echo "Node:        $(node --version)"
    @echo "NPM:         $(npm --version)"
    @echo "Docker:      $(docker --version)"
    @echo "SQLx:        $(sqlx --version 2>&1 || echo 'Not installed')"
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    @echo "Backend URL: http://localhost:8000"
    @echo "Frontend URL: http://localhost:3000"
    @echo "MinIO Console: http://localhost:9001"
    @echo "Database: postgresql://localhost:5432/bdp"
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check database connection
check-db:
    @echo "🔍 Checking database connection..."
    @psql ${DATABASE_URL} -c "SELECT version();" > /dev/null && echo "✓ Database connected" || echo "✗ Database connection failed"

# Show logs for all services
logs:
    docker compose logs -f

# Follow backend logs
logs-backend:
    docker compose logs -f bdp-server

# Follow frontend logs
logs-frontend:
    docker compose logs -f web

# Health check all services
health:
    @echo "🏥 Checking service health..."
    @curl -s http://localhost:8000/health > /dev/null && echo "✓ Backend healthy" || echo "✗ Backend down"
    @curl -s http://localhost:3000 > /dev/null && echo "✓ Frontend healthy" || echo "✗ Frontend down"
    @curl -s http://localhost:9000/minio/health/live > /dev/null && echo "✓ MinIO healthy" || echo "✗ MinIO down"

# ============================================================================
# Audit Logs (CQRS)
# ============================================================================

# View recent audit logs
audit-logs LIMIT="50":
    @echo "📋 Viewing recent audit logs (limit: {{LIMIT}})..."
    @psql ${DATABASE_URL} -c "SELECT id, timestamp, action, resource_type, resource_id, user_id FROM audit_log ORDER BY timestamp DESC LIMIT {{LIMIT}};"

# Search audit logs by action
audit-search TERM:
    @echo "🔍 Searching audit logs for: {{TERM}}"
    @psql ${DATABASE_URL} -c "SELECT id, timestamp, action, resource_type, resource_id, user_id, changes FROM audit_log WHERE action ILIKE '%{{TERM}}%' OR resource_type ILIKE '%{{TERM}}%' OR changes::text ILIKE '%{{TERM}}%' ORDER BY timestamp DESC LIMIT 50;"

# View audit logs for a specific resource type
audit-by-resource TYPE:
    @echo "📋 Viewing audit logs for resource type: {{TYPE}}"
    @psql ${DATABASE_URL} -c "SELECT id, timestamp, action, resource_type, resource_id, user_id, changes FROM audit_log WHERE resource_type = '{{TYPE}}' ORDER BY timestamp DESC LIMIT 50;"

# View audit logs for a specific user
audit-by-user USER_ID:
    @echo "📋 Viewing audit logs for user: {{USER_ID}}"
    @psql ${DATABASE_URL} -c "SELECT id, timestamp, action, resource_type, resource_id, changes FROM audit_log WHERE user_id = '{{USER_ID}}'::uuid ORDER BY timestamp DESC LIMIT 50;"

# View audit trail for a specific resource
audit-trail RESOURCE_TYPE RESOURCE_ID:
    @echo "📋 Viewing audit trail for {{RESOURCE_TYPE}} {{RESOURCE_ID}}"
    @psql ${DATABASE_URL} -c "SELECT id, timestamp, action, user_id, changes, metadata FROM audit_log WHERE resource_type = '{{RESOURCE_TYPE}}' AND resource_id = '{{RESOURCE_ID}}'::uuid ORDER BY timestamp ASC;"

# Export audit logs to JSON
audit-export OUTPUT="audit_logs.json":
    @echo "💾 Exporting audit logs to {{OUTPUT}}..."
    @psql ${DATABASE_URL} -t -A -F"," -c "SELECT row_to_json(t) FROM (SELECT id, timestamp, action, resource_type, resource_id, user_id, changes, metadata, ip_address FROM audit_log ORDER BY timestamp DESC LIMIT 1000) t;" > {{OUTPUT}}
    @echo "✓ Exported to {{OUTPUT}}"

# Show audit statistics
audit-stats:
    @echo "📊 Audit Log Statistics"
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    @psql ${DATABASE_URL} -c "SELECT action, COUNT(*) as count FROM audit_log GROUP BY action ORDER BY count DESC;"
    @echo ""
    @psql ${DATABASE_URL} -c "SELECT resource_type, COUNT(*) as count FROM audit_log GROUP BY resource_type ORDER BY count DESC;"
    @echo ""
    @psql ${DATABASE_URL} -c "SELECT DATE(timestamp) as date, COUNT(*) as count FROM audit_log GROUP BY DATE(timestamp) ORDER BY date DESC LIMIT 7;"

# ============================================================================
# End-to-End Testing
# ============================================================================

# Run E2E tests in CI mode (fast, uses committed fixtures)
e2e-ci:
    @Write-Host "🧪 Running E2E tests (CI mode)..."
    @$env:BDP_E2E_MODE = "ci"
    @cargo test --test e2e -- --test-threads=1 --nocapture

# Run E2E tests in Real mode (uses downloaded data)
e2e-real:
    @Write-Host "🧪 Running E2E tests (Real mode with downloaded data)..."
    @$env:BDP_E2E_MODE = "real"
    @cargo test --test e2e -- --test-threads=1 --nocapture

# Download real UniProt test data (idempotent, cached)
e2e-download-data:
    @Write-Host "📥 Downloading real UniProt test data..."
    @cargo run --bin download-test-data

# Run E2E tests with full observability output
e2e-debug:
    @Write-Host "🔍 Running E2E tests (debug mode)..."
    @$env:BDP_E2E_MODE = "ci"
    @$env:RUST_LOG = "debug,bdp_server=trace"
    @cargo test --test e2e -- --test-threads=1 --nocapture

# Clean E2E test data (removes downloaded data, keeps CI fixtures)
e2e-clean:
    @Write-Host "🧹 Cleaning E2E test data..."
    @if (Test-Path "tests/fixtures/real") { Remove-Item -Recurse -Force "tests/fixtures/real/*" -Exclude ".gitkeep" }
    @Write-Host "✓ E2E test data cleaned"

# Show E2E test data info
e2e-info:
    @Write-Host "📊 E2E Test Data Information"
    @Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    @Write-Host "CI Mode:"
    @if (Test-Path "tests/fixtures/uniprot_ci_sample.dat") { $size = (Get-Item "tests/fixtures/uniprot_ci_sample.dat").Length; Write-Host "  ✓ CI sample:     $([math]::Round($size/1KB, 1)) KB" } else { Write-Host "  ✗ CI sample not found" }
    @Write-Host ""
    @Write-Host "Real Mode:"
    @if (Test-Path "tests/fixtures/real") { $files = Get-ChildItem "tests/fixtures/real" -Filter "*.dat*"; if ($files.Count -gt 0) { foreach ($f in $files) { $size = $f.Length; Write-Host "  ✓ $($f.Name):  $([math]::Round($size/1MB, 1)) MB" } } else { Write-Host "  ⚠ No real data downloaded (run: just e2e-download-data)" } } else { Write-Host "  ✗ Real data directory not found" }
    @Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# ============================================================================
# Version Management & Releases
# ============================================================================

# Show current version
version:
    @echo "📦 BDP Version Information"
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    @cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.name=="bdp-cli") | "Rust:    v" + .version'
    @cd web && node -p "'Node:    v' + require('./package.json').version"
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Bump patch version (0.1.0 → 0.1.1) and create git tag
release-patch:
    @echo "📦 Bumping patch version..."
    cargo release patch --execute --no-publish

# Bump minor version (0.1.0 → 0.2.0) and create git tag
release-minor:
    @echo "📦 Bumping minor version..."
    cargo release minor --execute --no-publish

# Bump major version (0.1.0 → 1.0.0) and create git tag
release-major:
    @echo "📦 Bumping major version..."
    cargo release major --execute --no-publish

# Dry run of patch release (preview changes)
release-patch-dry:
    @echo "🔍 Dry run of patch release..."
    cargo release patch --no-publish

# Dry run of minor release (preview changes)
release-minor-dry:
    @echo "🔍 Dry run of minor release..."
    cargo release minor --no-publish

# Manual version bump without git operations (for testing)
bump-version VERSION:
    @echo "📦 Bumping version to {{VERSION}}..."
    @echo "Updating Cargo.toml..."
    @sed -i 's/^version = ".*"/version = "{{VERSION}}"/' Cargo.toml
    @echo "Syncing to package.json..."
    @NEW_VERSION={{VERSION}} node scripts/sync-version.js
    @echo "✓ Version bumped to {{VERSION}}"
    @echo "⚠️  Remember to commit and tag manually!"

# Install cargo-release if not already installed
install-cargo-release:
    @command -v cargo-release > /dev/null || (echo "Installing cargo-release..." && cargo install cargo-release)
    @echo "✓ cargo-release installed"
