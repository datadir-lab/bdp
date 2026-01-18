# BDP Setup Guide

Welcome to the BDP (Biological Data Platform) setup guide. This document will help you get your development environment ready to work on the BDP project.

## Table of Contents

- [System Requirements](#system-requirements)
- [Installation Steps](#installation-steps)
- [First-Time Setup](#first-time-setup)
- [Verification](#verification)
- [Common Issues](#common-issues)
- [Next Steps](#next-steps)

## System Requirements

### Required Software

The following software must be installed on your system:

#### 1. Docker (v24.0.0 or later)
Docker is used for running PostgreSQL, MinIO (S3-compatible storage), and other services.

- **Installation**: [https://docs.docker.com/get-docker/](https://docs.docker.com/get-docker/)
- **Verification**: `docker --version`
- **Note**: On Linux, ensure your user is in the `docker` group to run Docker without sudo

**Docker Compose** (usually included with Docker Desktop):
- **Verification**: `docker compose version` or `docker-compose --version`

#### 2. Rust Toolchain (1.75.0 or later)
Rust is the primary programming language for the BDP backend services.

- **Installation**: [https://rustup.rs/](https://rustup.rs/)
- **Installation command**:
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Verification**: `rustc --version` and `cargo --version`

#### 3. Node.js (v18.0.0 or later)
Node.js is required for the web frontend built with Next.js.

- **Installation**: [https://nodejs.org/](https://nodejs.org/) (LTS version recommended)
- **Verification**: `node --version` and `npm --version`
- **Note**: We recommend using [nvm](https://github.com/nvm-sh/nvm) (Node Version Manager) for managing Node.js versions

#### 4. Just Command Runner
Just is used as a command runner to replace shell scripts with a cross-platform solution.

- **Installation**:
  ```bash
  cargo install just
  ```
- **Verification**: `just --version`

#### 5. SQLx CLI
SQLx CLI is required for running database migrations.

- **Installation**:
  ```bash
  cargo install sqlx-cli --no-default-features --features rustls,postgres
  ```
- **Verification**: `sqlx --version`

### Recommended Software

These tools are not required but highly recommended:

- **Git** (v2.30.0 or later): Version control
  - Installation: [https://git-scm.com/downloads](https://git-scm.com/downloads)

- **PostgreSQL Client (psql)**: Direct database access for debugging
  - Installation: Included with PostgreSQL installation
  - Verification: `psql --version`

- **curl**: API testing and HTTP requests
  - Usually pre-installed on Linux/macOS
  - Windows: Available in Git Bash or WSL

- **jq**: JSON processing in command line
  - Installation: [https://stedolan.github.io/jq/download/](https://stedolan.github.io/jq/download/)

### Hardware Requirements

- **CPU**: 2+ cores recommended
- **RAM**: 8GB minimum, 16GB recommended
- **Disk Space**: 10GB free space for dependencies and Docker images

### Supported Operating Systems

- **Linux**: Ubuntu 20.04+, Debian 11+, Fedora 36+, or other modern distributions
- **macOS**: 11 (Big Sur) or later
- **Windows**: Windows 10/11 with WSL 2 (recommended) or native Windows

## Installation Steps

### Step 1: Clone the Repository

```bash
git clone https://github.com/datadir-lab/bdp.git
cd bdp
```

### Step 2: Install Required Dependencies

Follow the installation links in the [System Requirements](#system-requirements) section above.

**Quick verification** of all dependencies:
```bash
just verify
```

This command will check all required software and report their versions.

### Step 3: Install All Dependencies

Use the Just command to install all required dependencies:

```bash
just install-deps
```

This will:
- Install SQLx CLI for database migrations
- Install Node.js dependencies for the frontend

## First-Time Setup

### Quick Start (Recommended)

The easiest way to set up your development environment is to use the setup command:

```bash
just setup
```

This command will:
1. Install all dependencies
2. Set up environment variables (copy `.env.example` to `.env`)
3. Start all Docker services (PostgreSQL, MinIO, etc.)
4. Wait for databases to be ready
5. Run database migrations

### Manual Setup

If you prefer to set up manually or if the quick setup fails:

#### 1. Configure Environment Variables

```bash
just env-setup
```

This copies `.env.example` to `.env`. Edit `.env` and update any values as needed.

#### 2. Start Docker Services

```bash
just db-up
```

This starts PostgreSQL and waits for it to be ready.

#### 3. Run Database Migrations

```bash
just db-migrate
```

If you see "Migrations complete", the database is ready!

#### 4. Verify Setup

```bash
just verify
```

## Verification

### Checklist

After completing the setup, verify the following:

- [ ] All required software installed (run `just verify`)
- [ ] `.env` file exists and is configured
- [ ] Docker services are running (`just db-up`)
- [ ] Database migrations applied successfully
- [ ] Frontend dependencies installed

### Verification Commands

```bash
# Comprehensive verification
just verify

# Check environment info
just info

# Check database connection
just check-db

# View database logs
just db-logs
```

## Common Issues

### Issue: Docker daemon not running

**Symptoms**: `Cannot connect to the Docker daemon`

**Solution**:
- **Linux**: `sudo systemctl start docker`
- **macOS/Windows**: Start Docker Desktop application

### Issue: Port already in use

**Symptoms**: `port is already allocated` or `address already in use`

**Solution**:
1. Check which ports are in use:
   ```bash
   # Linux/macOS
   sudo lsof -i :5432  # PostgreSQL
   sudo lsof -i :9000  # MinIO
   sudo lsof -i :8000  # Backend API

   # Windows (PowerShell)
   netstat -ano | findstr :5432
   ```

2. Stop conflicting services or change ports in `.env`:
   ```env
   POSTGRES_PORT=5433
   MINIO_PORT=9001
   SERVER_PORT=8001
   ```

3. Restart Docker services:
   ```bash
   docker compose down
   docker compose up -d
   ```

### Issue: just or sqlx-cli not found

**Symptoms**: `command not found: just` or `command not found: sqlx`

**Solution**:
```bash
# Install both tools
cargo install just sqlx-cli --features postgres

# Ensure cargo bin directory is in PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Add to shell profile (~/.bashrc, ~/.zshrc, etc.)
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Issue: Database connection refused

**Symptoms**: `Connection refused` when running migrations or connecting to database

**Solution**:
1. Start the database:
   ```bash
   just db-up
   ```

2. Check database logs:
   ```bash
   just db-logs
   ```

3. Verify database connection:
   ```bash
   just check-db
   ```

4. Verify DATABASE_URL in `.env` matches Docker configuration:
   ```env
   DATABASE_URL=postgresql://bdp:bdp_dev_password@localhost:5432/bdp
   ```

### Issue: Just command not working

**Symptoms**: Just commands fail or are not recognized

**Solution**:
```bash
# Verify Just is installed
just --version

# If not installed
cargo install just

# See all available commands
just --list
```

### Issue: Node.js version too old

**Symptoms**: Errors during `npm install` or `npm run dev`

**Solution**:
```bash
# Check current version
node --version

# Install nvm (Node Version Manager)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash

# Install and use latest LTS
nvm install --lts
nvm use --lts

# Verify
node --version  # Should be v18.x or later
```

### Issue: Rust compilation errors

**Symptoms**: Build failures or compiler errors

**Solution**:
```bash
# Update Rust toolchain
rustup update

# Clean build artifacts
cargo clean

# Rebuild
cargo build
```

### Issue: Docker containers keep restarting

**Symptoms**: Services show "restarting" status in `docker compose ps`

**Solution**:
1. Check container logs:
   ```bash
   docker compose logs <service-name>
   ```

2. Common causes:
   - Port conflicts (see "Port already in use" above)
   - Insufficient memory (increase Docker memory limit)
   - Corrupted volumes (remove and recreate):
     ```bash
     docker compose down -v
     docker compose up -d
     ```

### Issue: MinIO bucket not created

**Symptoms**: S3 operations fail with "bucket does not exist"

**Solution**:
```bash
# Check minio-init container logs
docker compose logs minio-init

# Manually create bucket
docker compose exec minio mc alias set local http://localhost:9000 minioadmin minioadmin
docker compose exec minio mc mb local/bdp-data --ignore-existing
```

## Next Steps

After successful setup, you can:

### 1. Run the Backend Server

```bash
# Start development server
just dev
```

The API will be available at [http://localhost:8000](http://localhost:8000)

### 2. Run the Frontend

```bash
# Start frontend development server
just web
```

The web interface will be available at [http://localhost:3000](http://localhost:3000)

### 3. Run All Services

```bash
# Start backend + frontend + database together
just dev-all
```

### 4. Run Tests

```bash
# Run all tests
just test

# Run with verbose output
just test-verbose

# Run specific test
just test-one test_name
```

### 4. Explore the Documentation

- **API Documentation**: See `docs/api/` for API endpoint documentation
- **Development Guide**: See `docs/development/` for coding standards and workflows
- **Architecture**: See `docs/architecture/` for system design documentation

### 5. Development Workflow

```bash
# 1. Start all services
just setup      # First time only

# 2. Start development
just dev        # Backend (in one terminal)
just web        # Frontend (in another terminal)

# Or start everything together
just dev-all

# 3. Make changes and test
just test       # Run tests
just lint       # Check code quality
just fmt        # Format code
```

### 6. Useful Commands

```bash
# View all available commands
just --list

# Database management
just db-up          # Start database
just db-down        # Stop database
just db-migrate     # Run migrations
just db-shell       # Access PostgreSQL shell
just db-logs        # View database logs
just db-reset       # Reset database (WARNING: deletes data)

# Development
just dev            # Start backend
just web            # Start frontend
just dev-all        # Start all services
just watch          # Watch and rebuild on changes

# Testing
just test           # Run all tests
just test-verbose   # Run tests with output
just test-unit      # Run unit tests only
just test-integration # Run integration tests only

# Code quality
just fmt            # Format code
just lint           # Run linters
just fix            # Auto-fix linting issues

# SQLx management
just sqlx-prepare   # Generate metadata for offline builds
just sqlx-check     # Verify metadata is up to date
just sqlx-clean     # Clean metadata files

# CI/CD simulation
just ci             # Run all CI checks locally
just ci-offline     # Run CI in offline mode

# Build
just build          # Build backend
just build-release  # Build release version
just build-web      # Build frontend
just build-all      # Build everything

# Utilities
just info           # Show environment info
just verify         # Verify setup
just check-db       # Check database connection
just clean          # Clean build artifacts
```

## Getting Help

If you encounter issues not covered in this guide:

1. **Check the documentation**: Look in `docs/` for more detailed guides
2. **Review logs**: Check logs with `just db-logs` or `just logs`
3. **Run verification**: `just verify` to identify missing dependencies
4. **Search issues**: Check GitHub issues for similar problems
5. **Ask for help**: Create a new GitHub issue with:
   - Output of `just verify` and `just info`
   - Relevant error messages
   - Your OS and software versions
   - Steps to reproduce the issue

## Additional Resources

- [Docker Documentation](https://docs.docker.com/)
- [Rust Programming Language](https://www.rust-lang.org/learn)
- [SQLx Documentation](https://github.com/launchbadge/sqlx)
- [Next.js Documentation](https://nextjs.org/docs)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [MinIO Documentation](https://min.io/docs/)

---

**Happy coding!** If you have suggestions for improving this guide, please submit a pull request.
