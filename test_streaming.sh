#!/bin/bash
# Quick test script for GenBank streaming implementation

set -e

echo "==================================="
echo "GenBank Streaming Tests"
echo "==================================="
echo ""

cd "$(dirname "$0")/crates/bdp-server"

echo "1. Running unit tests..."
cargo test --lib ingest::genbank::ftp::tests --quiet -- --nocapture

echo ""
echo "2. Running streaming integration tests..."
cargo test --test genbank_streaming_test --quiet -- --nocapture

echo ""
echo "3. Running existing GenBank parser tests (regression)..."
cargo test --test genbank_parser_test --quiet -- --nocapture

echo ""
echo "4. Checking compilation..."
cargo check --lib --quiet

echo ""
echo "==================================="
echo "✓ All tests passed!"
echo "==================================="
echo ""
echo "To run benchmarks:"
echo "  cargo bench --bench genbank_streaming_bench"
echo ""
echo "To run E2E tests (requires Docker):"
echo "  cargo test --test genbank_e2e_streaming"
echo ""
