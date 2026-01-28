#!/bin/bash
# Run all search optimization tests and benchmarks
#
# Usage:
#   ./scripts/run_search_tests.sh [OPTIONS]
#
# Options:
#   --unit          Run unit tests only
#   --integration   Run integration tests only
#   --load          Run load tests only
#   --bench         Run benchmarks only
#   --quick         Quick test run (smaller datasets)
#   --full          Full test run (large datasets, slow)
#   --help          Show this help message

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default options
RUN_UNIT=false
RUN_INTEGRATION=false
RUN_LOAD=false
RUN_BENCH=false
QUICK_MODE=false

# Parse arguments
if [ $# -eq 0 ]; then
    # If no arguments, run all tests except load and bench
    RUN_UNIT=true
    RUN_INTEGRATION=true
else
    while [[ $# -gt 0 ]]; do
        case $1 in
            --unit)
                RUN_UNIT=true
                shift
                ;;
            --integration)
                RUN_INTEGRATION=true
                shift
                ;;
            --load)
                RUN_LOAD=true
                shift
                ;;
            --bench)
                RUN_BENCH=true
                shift
                ;;
            --quick)
                QUICK_MODE=true
                shift
                ;;
            --full)
                RUN_UNIT=true
                RUN_INTEGRATION=true
                RUN_LOAD=true
                RUN_BENCH=true
                shift
                ;;
            --help)
                head -n 15 "$0" | tail -n 14
                exit 0
                ;;
            *)
                echo -e "${RED}Unknown option: $1${NC}"
                echo "Run with --help for usage information"
                exit 1
                ;;
        esac
    done
fi

echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         Search Optimization Test Suite                       ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check environment
if [ -z "$DATABASE_URL" ]; then
    echo -e "${RED}✗ DATABASE_URL not set${NC}"
    echo "  Please set DATABASE_URL environment variable"
    exit 1
fi

echo -e "${GREEN}✓ DATABASE_URL configured${NC}"
echo ""

# Run migrations if needed
echo -e "${YELLOW}→ Checking database migrations...${NC}"
if command -v sqlx &> /dev/null; then
    sqlx migrate run || {
        echo -e "${RED}✗ Failed to run migrations${NC}"
        exit 1
    }
    echo -e "${GREEN}✓ Migrations applied${NC}"
else
    echo -e "${YELLOW}⚠ sqlx-cli not installed, skipping migration check${NC}"
fi
echo ""

# Unit tests
if [ "$RUN_UNIT" = true ]; then
    echo -e "${BLUE}══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  Running Unit Tests${NC}"
    echo -e "${BLUE}══════════════════════════════════════════════════════════════${NC}"
    echo ""

    cargo test --package bdp-server --lib features::search::queries --no-fail-fast || {
        echo -e "${RED}✗ Unit tests failed${NC}"
        exit 1
    }

    echo ""
    echo -e "${GREEN}✓ Unit tests passed${NC}"
    echo ""
fi

# Integration tests
if [ "$RUN_INTEGRATION" = true ]; then
    echo -e "${BLUE}══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  Running Integration Tests${NC}"
    echo -e "${BLUE}══════════════════════════════════════════════════════════════${NC}"
    echo ""

    cargo test --package bdp-server --test search_integration_tests --no-fail-fast -- --nocapture || {
        echo -e "${RED}✗ Integration tests failed${NC}"
        exit 1
    }

    echo ""
    echo -e "${GREEN}✓ Integration tests passed${NC}"
    echo ""
fi

# Load tests
if [ "$RUN_LOAD" = true ]; then
    echo -e "${BLUE}══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  Running Load Tests${NC}"
    echo -e "${BLUE}══════════════════════════════════════════════════════════════${NC}"
    echo ""
    echo -e "${YELLOW}⚠ Load tests may take several minutes...${NC}"
    echo ""

    # Run load tests one at a time to avoid overwhelming the database
    if [ "$QUICK_MODE" = true ]; then
        echo -e "${YELLOW}→ Running quick load test (concurrent searches only)${NC}"
        cargo test --package bdp-server --test search_load_tests test_concurrent_searches -- --ignored --nocapture --test-threads=1 || {
            echo -e "${RED}✗ Load tests failed${NC}"
            exit 1
        }
    else
        echo -e "${YELLOW}→ Running all load tests${NC}"
        cargo test --package bdp-server --test search_load_tests -- --ignored --nocapture --test-threads=1 || {
            echo -e "${RED}✗ Load tests failed${NC}"
            exit 1
        }
    fi

    echo ""
    echo -e "${GREEN}✓ Load tests passed${NC}"
    echo ""
fi

# Benchmarks
if [ "$RUN_BENCH" = true ]; then
    echo -e "${BLUE}══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  Running Benchmarks${NC}"
    echo -e "${BLUE}══════════════════════════════════════════════════════════════${NC}"
    echo ""
    echo -e "${YELLOW}⚠ Benchmarks may take 10-30 minutes...${NC}"
    echo ""

    if [ "$QUICK_MODE" = true ]; then
        echo -e "${YELLOW}→ Running quick benchmarks (sample size: 10)${NC}"
        cargo bench --bench search_performance -- --sample-size 10 || {
            echo -e "${RED}✗ Benchmarks failed${NC}"
            exit 1
        }
    else
        echo -e "${YELLOW}→ Running full benchmarks${NC}"
        cargo bench --bench search_performance || {
            echo -e "${RED}✗ Benchmarks failed${NC}"
            exit 1
        }
    fi

    echo ""
    echo -e "${GREEN}✓ Benchmarks completed${NC}"
    echo ""

    # Show benchmark report location
    REPORT_DIR="target/criterion"
    if [ -d "$REPORT_DIR" ]; then
        echo -e "${BLUE}Benchmark reports available at:${NC}"
        echo -e "  file://$(pwd)/$REPORT_DIR/report/index.html"
        echo ""
    fi
fi

# Summary
echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                    Test Summary                              ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

if [ "$RUN_UNIT" = true ]; then
    echo -e "${GREEN}✓ Unit tests: PASSED${NC}"
fi

if [ "$RUN_INTEGRATION" = true ]; then
    echo -e "${GREEN}✓ Integration tests: PASSED${NC}"
fi

if [ "$RUN_LOAD" = true ]; then
    echo -e "${GREEN}✓ Load tests: PASSED${NC}"
fi

if [ "$RUN_BENCH" = true ]; then
    echo -e "${GREEN}✓ Benchmarks: COMPLETED${NC}"
fi

echo ""
echo -e "${GREEN}All tests passed successfully! 🎉${NC}"
echo ""
