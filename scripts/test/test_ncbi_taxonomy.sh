#!/bin/bash
# Test script for NCBI Taxonomy integration tests
#
# This script runs the NCBI taxonomy integration tests against a PostgreSQL database.
# It requires a running PostgreSQL instance with migrations applied.
#
# Usage:
#   ./test_ncbi_taxonomy.sh [options]
#
# Options:
#   --unit-only       Run only unit tests (parser, pipeline, version discovery)
#   --integration     Run integration tests (requires database)
#   --all             Run all tests (default)
#   --nocapture       Show test output (useful for debugging)
#
# Environment:
#   DATABASE_URL      PostgreSQL connection string (default: postgresql://localhost/bdp_test)

set -e

# Default options
RUN_UNIT=true
RUN_INTEGRATION=false
NOCAPTURE=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --unit-only)
            RUN_UNIT=true
            RUN_INTEGRATION=false
            shift
            ;;
        --integration)
            RUN_UNIT=false
            RUN_INTEGRATION=true
            shift
            ;;
        --all)
            RUN_UNIT=true
            RUN_INTEGRATION=true
            shift
            ;;
        --nocapture)
            NOCAPTURE="--nocapture"
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--unit-only|--integration|--all] [--nocapture]"
            exit 1
            ;;
    esac
done

# Color codes for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}NCBI Taxonomy Test Suite${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Check if DATABASE_URL is set for integration tests
if [ "$RUN_INTEGRATION" = true ]; then
    if [ -z "$DATABASE_URL" ]; then
        echo -e "${YELLOW}Warning: DATABASE_URL not set, using default: postgresql://localhost/bdp_test${NC}"
        export DATABASE_URL="postgresql://localhost/bdp_test"
    fi
    echo -e "${BLUE}Database:${NC} $DATABASE_URL"
    echo ""
fi

# Run unit tests
if [ "$RUN_UNIT" = true ]; then
    echo -e "${GREEN}Running Unit Tests...${NC}"
    echo -e "${BLUE}1. Parser tests (12 tests)${NC}"
    cargo test --test ncbi_taxonomy_parser_test $NOCAPTURE

    echo ""
    echo -e "${BLUE}2. Pipeline tests (2 tests)${NC}"
    cargo test --lib ncbi_taxonomy::pipeline::tests $NOCAPTURE

    echo ""
    echo -e "${BLUE}3. Version discovery tests${NC}"
    cargo test --lib ncbi_taxonomy::version_discovery::tests $NOCAPTURE

    echo ""
    echo -e "${GREEN}✓ Unit tests completed${NC}"
    echo ""
fi

# Run integration tests
if [ "$RUN_INTEGRATION" = true ]; then
    echo -e "${GREEN}Running Integration Tests (8 tests)...${NC}"
    echo -e "${YELLOW}Note: These require a running PostgreSQL database with migrations applied${NC}"
    echo ""

    cargo test --test ncbi_taxonomy_integration_test -- --ignored $NOCAPTURE

    echo ""
    echo -e "${GREEN}✓ Integration tests completed${NC}"
    echo ""
fi

echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}All tests completed successfully!${NC}"
echo -e "${BLUE}========================================${NC}"
