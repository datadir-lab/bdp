#!/bin/bash
# Rebuild BDP Backend Server
# This script prepares SQLx cache, rebuilds the Docker image, and restarts the server

set -e

echo "=========================================="
echo "BDP Backend Rebuild Script"
echo "=========================================="
echo ""

# Get the root directory
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Load environment variables
if [ -f .env.docker ]; then
    echo "Loading environment from .env.docker..."
    export $(cat .env.docker | grep -v '^#' | xargs)
fi

# Step 1: Check if database is running
echo "Step 1: Checking database..."
if ! docker ps --filter "name=bdp-postgres" --format "{{.Names}}" | grep -q "bdp-postgres"; then
    echo "Error: PostgreSQL container is not running!"
    echo "Please start it with: docker-compose up -d postgres"
    exit 1
fi
echo "✓ Database is running"
echo ""

# Step 2: Prepare SQLx offline cache
echo "Step 2: Preparing SQLx offline cache..."
export DATABASE_URL="postgresql://${POSTGRES_USER:-bdp}:${POSTGRES_PASSWORD:-bdp_dev_password}@localhost:${POSTGRES_PORT:-5432}/${POSTGRES_DB:-bdp}"

# Check if sqlx-cli is installed
if ! command -v cargo-sqlx &> /dev/null; then
    echo "Installing sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features postgres
fi

# Prepare the query cache
echo "Generating query cache..."
cd "$ROOT_DIR"
cargo sqlx prepare --workspace -- --all-targets

echo "✓ SQLx cache prepared"
echo ""

# Step 3: Stop the current server
echo "Step 3: Stopping current server..."
docker-compose stop bdp-server || true
echo "✓ Server stopped"
echo ""

# Step 4: Rebuild the Docker image
echo "Step 4: Building new Docker image..."
docker-compose build --no-cache bdp-server
echo "✓ Docker image built"
echo ""

# Step 5: Start the server
echo "Step 5: Starting server..."
docker-compose up -d bdp-server
echo "✓ Server started"
echo ""

# Step 6: Wait for health check
echo "Step 6: Waiting for server to be healthy..."
max_attempts=30
attempt=0

while [ $attempt -lt $max_attempts ]; do
    if curl -sf http://localhost:8000/health > /dev/null 2>&1; then
        echo "✓ Server is healthy!"
        break
    fi

    attempt=$((attempt + 1))
    if [ $attempt -eq $max_attempts ]; then
        echo "Warning: Server health check timeout"
        echo "Check logs with: docker-compose logs bdp-server"
        exit 1
    fi

    echo "Waiting for server... ($attempt/$max_attempts)"
    sleep 2
done
echo ""

# Step 7: Show logs
echo "=========================================="
echo "Server Status:"
echo "=========================================="
docker-compose ps bdp-server
echo ""

echo "Recent logs:"
docker-compose logs --tail=20 bdp-server
echo ""

echo "=========================================="
echo "✓ Backend rebuild complete!"
echo "=========================================="
echo ""
echo "Useful commands:"
echo "  - View logs: docker-compose logs -f bdp-server"
echo "  - Check status: docker-compose ps"
echo "  - Test API: curl http://localhost:8000/health"
echo ""
