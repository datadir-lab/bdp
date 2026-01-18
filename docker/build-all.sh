#!/bin/bash
# Build all BDP Docker images
# Usage: ./docker/build-all.sh [tag]

set -e

TAG="${1:-latest}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Building all BDP Docker images with tag: $TAG"
echo "Root directory: $ROOT_DIR"
echo ""

# Build server
echo "Building bdp-server..."
docker build -f "$ROOT_DIR/docker/Dockerfile.server" \
  -t "bdp-server:$TAG" \
  "$ROOT_DIR"
echo "✓ bdp-server:$TAG built successfully"
echo ""

# Build CLI
echo "Building bdp-cli..."
docker build -f "$ROOT_DIR/docker/Dockerfile.cli" \
  -t "bdp-cli:$TAG" \
  "$ROOT_DIR"
echo "✓ bdp-cli:$TAG built successfully"
echo ""

# Build ingest
echo "Building bdp-ingest..."
docker build -f "$ROOT_DIR/docker/Dockerfile.ingest" \
  -t "bdp-ingest:$TAG" \
  "$ROOT_DIR"
echo "✓ bdp-ingest:$TAG built successfully"
echo ""

# Build web
echo "Building bdp-web..."
docker build -f "$ROOT_DIR/docker/Dockerfile.web" \
  -t "bdp-web:$TAG" \
  "$ROOT_DIR"
echo "✓ bdp-web:$TAG built successfully"
echo ""

echo "All images built successfully!"
echo ""
echo "Images:"
docker images | grep "bdp-" | grep "$TAG"
