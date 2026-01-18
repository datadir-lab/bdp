#!/bin/bash
# Test UniProt Ingestion System

echo "=== UniProt Ingestion Test ==="
echo ""

# Database connection
export DATABASE_URL="postgresql://bdp:bdp_dev_password@localhost:5432/bdp"

echo "1. Checking database connectivity..."
docker compose exec postgres psql -U bdp -d bdp -c "SELECT COUNT(*) FROM organizations;" > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "   ✓ Database connected"
else
    echo "   ✗ Database connection failed"
    exit 1
fi

echo ""
echo "2. Checking ingestion tables..."
TABLES="ingestion_jobs ingestion_raw_files ingestion_work_units organization_sync_status"
for table in $TABLES; do
    docker compose exec postgres psql -U bdp -d bdp -c "SELECT COUNT(*) FROM $table;" > /dev/null 2>&1
    if [ $? -eq 0 ]; then
        echo "   ✓ Table $table exists"
    else
        echo "   ✗ Table $table missing"
    fi
done

echo ""
echo "3. Checking version discovery tables..."
docker compose exec postgres psql -U bdp -d bdp -c "SELECT COUNT(*) FROM versions;" > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "   ✓ Versions table exists"
fi

echo ""
echo "4. Configuration check..."
echo "   Mode: ${INGEST_UNIPROT_MODE:-latest}"
echo "   Enabled: ${INGEST_ENABLED:-false}"

echo ""
echo "=== Test Complete ==="
echo ""
echo "To enable ingestion:"
echo "  export INGEST_ENABLED=true"
echo "  export INGEST_UNIPROT_MODE=latest"
