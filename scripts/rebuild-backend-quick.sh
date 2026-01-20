#!/bin/bash
# Quick Rebuild BDP Backend Server
# This script rebuilds the Docker image and restarts the server
# Skips SQLx cache generation (uses existing cache)

set -e

echo "=========================================="
echo "BDP Backend Quick Rebuild"
echo "=========================================="
echo ""

# Get the root directory
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Step 1: Stop the current server
echo "Step 1: Stopping current server..."
docker-compose stop bdp-server
echo "✓ Server stopped"
echo ""

# Step 2: Rebuild the Docker image
echo "Step 2: Building new Docker image..."
docker-compose build --no-cache bdp-server
echo "✓ Docker image built"
echo ""

# Step 3: Start the server
echo "Step 3: Starting server..."
docker-compose up -d bdp-server
echo "✓ Server started"
echo ""

# Step 4: Wait for health check
echo "Step 4: Waiting for server to be healthy..."
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

# Step 5: Show logs
echo "=========================================="
echo "Server Status:"
echo "=========================================="
docker-compose ps bdp-server
echo ""

echo "Recent logs:"
docker-compose logs --tail=30 bdp-server
echo ""

echo "=========================================="
echo "✓ Backend rebuild complete!"
echo "=========================================="
echo ""
echo "Test the new endpoint:"
echo "  curl http://localhost:8000/api/v1/data-sources/source-types"
echo ""
