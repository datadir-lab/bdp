#!/bin/bash
# Reset Docker database and restart ingestion
# This script stops containers, removes volumes, and starts fresh

set -e  # Exit on any error

echo "================================================"
echo "BDP Docker Database Reset & Ingestion Restart"
echo "================================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Change to project root
cd "$(dirname "$0")/.."
PROJECT_ROOT=$(pwd)

echo -e "${YELLOW}[1/7] Stopping all containers...${NC}"
docker-compose down

echo ""
echo -e "${YELLOW}[2/7] Removing database volumes...${NC}"
docker volume rm bdp_postgres_data 2>/dev/null || echo "Volume bdp_postgres_data doesn't exist (OK)"
docker volume rm bdp_postgres_test_data 2>/dev/null || echo "Volume bdp_postgres_test_data doesn't exist (OK)"

echo ""
echo -e "${YELLOW}[3/7] Starting PostgreSQL and MinIO...${NC}"
docker-compose up -d postgres minio minio-init

echo ""
echo -e "${YELLOW}[4/7] Waiting for PostgreSQL to be ready...${NC}"
echo "This may take 10-15 seconds..."
sleep 5

# Wait for PostgreSQL to be healthy
for i in {1..30}; do
    if docker exec bdp-postgres pg_isready -U bdp -d bdp > /dev/null 2>&1; then
        echo -e "${GREEN}✓ PostgreSQL is ready${NC}"
        break
    fi

    if [ $i -eq 30 ]; then
        echo -e "${RED}✗ PostgreSQL failed to start${NC}"
        exit 1
    fi

    echo -n "."
    sleep 1
done

echo ""
echo -e "${YELLOW}[5/7] Running database migrations...${NC}"
echo "Using DATABASE_URL from .env.docker..."

# Source the .env.docker file to get connection details
if [ -f ".env.docker" ]; then
    export $(grep -v '^#' .env.docker | xargs)
else
    echo -e "${RED}Error: .env.docker file not found${NC}"
    exit 1
fi

# Set DATABASE_URL for local machine connection
export DATABASE_URL="postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@localhost:${POSTGRES_PORT}/${POSTGRES_DB}"

echo "Running: sqlx migrate run"
sqlx migrate run --source migrations

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Migrations completed successfully${NC}"
else
    echo -e "${RED}✗ Migration failed${NC}"
    exit 1
fi

echo ""
echo -e "${YELLOW}[6/7] Rebuilding bdp-server Docker image...${NC}"
docker-compose build bdp-server

echo ""
echo -e "${YELLOW}[7/7] Starting bdp-server with ingestion enabled...${NC}"
docker-compose --env-file .env.docker up -d bdp-server

echo ""
echo "================================================"
echo -e "${GREEN}✓ Database reset and ingestion restart complete!${NC}"
echo "================================================"
echo ""
echo "Next steps:"
echo "  1. Monitor logs:"
echo "     docker-compose logs -f bdp-server"
echo ""
echo "  2. Check database:"
echo "     docker exec -it bdp-postgres psql -U bdp -d bdp"
echo ""
echo "  3. Trigger ingestion (if not auto-started):"
echo "     curl -X POST http://localhost:8000/api/v1/ingest/uniprot/trigger"
echo ""
echo "Expected improvements:"
echo "  - TAR cache: Single decompression (2,280x faster)"
echo "  - Batch operations: 300-500x query reduction"
echo "  - Worker count: 16 workers (4x throughput)"
echo "  - Batch size: 5000 entries per batch (5x efficiency)"
echo ""
echo "Monitor for:"
echo "  - ERROR level messages (root cause of storage failures)"
echo "  - 'Cache hit' logs (TAR decompression cache)"
echo "  - 'Organism cache hit' logs"
echo "  - 'Batch inserting' logs"
echo "  - 'Creating organism bundle' logs"
echo ""
