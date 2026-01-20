#!/bin/bash

# Generate database schema diagram from PostgreSQL database
#
# Requirements:
# - postgresql-autodoc (install: apt-get install postgresql-autodoc)
# - graphviz (install: apt-get install graphviz)
# - Or use SchemaSpy: https://schemaspy.org/

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/output"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# Database connection details from environment or defaults
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-bdp}"
DB_USER="${DB_USER:-bdp}"
DB_PASSWORD="${DB_PASSWORD:-bdp_dev_password}"

echo "Generating database diagram..."
echo "Database: $DB_NAME@$DB_HOST:$DB_PORT"

# Method 1: Using pg_dump to create SQL schema
echo "Generating SQL schema..."
docker exec bdp-postgres pg_dump -U "$DB_USER" -d "$DB_NAME" --schema-only \
    > "$OUTPUT_DIR/schema_$TIMESTAMP.sql"

# Method 2: Using psql to generate table information
echo "Generating table list..."
docker exec bdp-postgres psql -U "$DB_USER" -d "$DB_NAME" -c "\dt" \
    > "$OUTPUT_DIR/tables_$TIMESTAMP.txt"

# Method 3: Generate ER diagram data
echo "Generating ER diagram data..."

# Get all tables
docker exec bdp-postgres psql -U "$DB_USER" -d "$DB_NAME" -t -c "
SELECT table_name
FROM information_schema.tables
WHERE table_schema = 'public'
  AND table_type = 'BASE TABLE'
ORDER BY table_name;
" > "$OUTPUT_DIR/table_list.txt"

# Generate table details with columns and relationships
docker exec bdp-postgres psql -U "$DB_USER" -d "$DB_NAME" -t -c "
SELECT
    tc.table_name,
    c.column_name,
    c.data_type,
    c.is_nullable,
    CASE
        WHEN pk.column_name IS NOT NULL THEN 'PK'
        WHEN fk.column_name IS NOT NULL THEN 'FK'
        ELSE ''
    END as key_type,
    fk.foreign_table_name,
    fk.foreign_column_name
FROM information_schema.tables tc
JOIN information_schema.columns c
    ON tc.table_name = c.table_name
    AND tc.table_schema = c.table_schema
LEFT JOIN (
    SELECT ku.table_name, ku.column_name
    FROM information_schema.table_constraints AS tc
    JOIN information_schema.key_column_usage AS ku
        ON tc.constraint_name = ku.constraint_name
        AND tc.table_schema = ku.table_schema
    WHERE tc.constraint_type = 'PRIMARY KEY'
) pk ON c.table_name = pk.table_name AND c.column_name = pk.column_name
LEFT JOIN (
    SELECT
        ku.table_name,
        ku.column_name,
        ccu.table_name AS foreign_table_name,
        ccu.column_name AS foreign_column_name
    FROM information_schema.table_constraints AS tc
    JOIN information_schema.key_column_usage AS ku
        ON tc.constraint_name = ku.constraint_name
        AND tc.table_schema = ku.table_schema
    JOIN information_schema.constraint_column_usage AS ccu
        ON ccu.constraint_name = tc.constraint_name
        AND ccu.table_schema = tc.table_schema
    WHERE tc.constraint_type = 'FOREIGN KEY'
) fk ON c.table_name = fk.table_name AND c.column_name = fk.column_name
WHERE tc.table_schema = 'public'
    AND tc.table_type = 'BASE TABLE'
ORDER BY tc.table_name, c.ordinal_position;
" > "$OUTPUT_DIR/table_details_$TIMESTAMP.txt"

# Generate DOT file for GraphViz
echo "Generating GraphViz DOT file..."

cat > "$OUTPUT_DIR/schema_$TIMESTAMP.dot" << 'EOF'
digraph schema {
    rankdir=LR;
    node [shape=record, style=filled, fillcolor=lightblue];

EOF

# Get all tables and their columns
docker exec bdp-postgres psql -U "$DB_USER" -d "$DB_NAME" -t -A -F'|' -c "
SELECT DISTINCT table_name
FROM information_schema.tables
WHERE table_schema = 'public' AND table_type = 'BASE TABLE'
ORDER BY table_name;
" | while IFS='|' read -r table; do
    [ -z "$table" ] && continue

    echo "    \"$table\" [label=\"{$table|" >> "$OUTPUT_DIR/schema_$TIMESTAMP.dot"

    # Get columns for this table
    docker exec bdp-postgres psql -U "$DB_USER" -d "$DB_NAME" -t -A -F'|' -c "
    SELECT
        c.column_name || ' : ' || c.data_type ||
        CASE WHEN pk.column_name IS NOT NULL THEN ' PK' ELSE '' END
    FROM information_schema.columns c
    LEFT JOIN (
        SELECT ku.table_name, ku.column_name
        FROM information_schema.table_constraints AS tc
        JOIN information_schema.key_column_usage AS ku
            ON tc.constraint_name = ku.constraint_name
            AND tc.table_schema = ku.table_schema
        WHERE tc.constraint_type = 'PRIMARY KEY'
    ) pk ON c.table_name = pk.table_name AND c.column_name = pk.column_name
    WHERE c.table_name = '$table' AND c.table_schema = 'public'
    ORDER BY c.ordinal_position;
    " | sed 's/^/+ /' | tr '\n' '\\' | sed 's/\\$/}\"];\n/' >> "$OUTPUT_DIR/schema_$TIMESTAMP.dot"
done

# Add foreign key relationships
docker exec bdp-postgres psql -U "$DB_USER" -d "$DB_NAME" -t -A -F'|' -c "
SELECT
    tc.table_name,
    ccu.table_name AS foreign_table
FROM information_schema.table_constraints AS tc
JOIN information_schema.constraint_column_usage AS ccu
    ON ccu.constraint_name = tc.constraint_name
WHERE tc.constraint_type = 'FOREIGN KEY';
" | while IFS='|' read -r from_table to_table; do
    [ -z "$from_table" ] && continue
    echo "    \"$from_table\" -> \"$to_table\";" >> "$OUTPUT_DIR/schema_$TIMESTAMP.dot"
done

echo "}" >> "$OUTPUT_DIR/schema_$TIMESTAMP.dot"

# Generate PNG from DOT file (if graphviz is available)
if command -v dot &> /dev/null; then
    echo "Generating PNG diagram..."
    dot -Tpng "$OUTPUT_DIR/schema_$TIMESTAMP.dot" -o "$OUTPUT_DIR/schema_$TIMESTAMP.png"
    echo "PNG diagram generated: $OUTPUT_DIR/schema_$TIMESTAMP.png"
else
    echo "Graphviz 'dot' command not found. Install graphviz to generate PNG diagram."
    echo "You can visualize the DOT file online at: https://dreampuf.github.io/GraphvizOnline/"
fi

# Create symlinks to latest
ln -sf "schema_$TIMESTAMP.sql" "$OUTPUT_DIR/schema_latest.sql"
ln -sf "tables_$TIMESTAMP.txt" "$OUTPUT_DIR/tables_latest.txt"
ln -sf "table_details_$TIMESTAMP.txt" "$OUTPUT_DIR/table_details_latest.txt"
ln -sf "schema_$TIMESTAMP.dot" "$OUTPUT_DIR/schema_latest.dot"
if [ -f "$OUTPUT_DIR/schema_$TIMESTAMP.png" ]; then
    ln -sf "schema_$TIMESTAMP.png" "$OUTPUT_DIR/schema_latest.png"
fi

echo ""
echo "✅ Database diagram generation complete!"
echo "Output directory: $OUTPUT_DIR"
echo ""
echo "Files generated:"
echo "  - schema_$TIMESTAMP.sql (SQL schema dump)"
echo "  - tables_$TIMESTAMP.txt (Table list)"
echo "  - table_details_$TIMESTAMP.txt (Detailed table information)"
echo "  - schema_$TIMESTAMP.dot (GraphViz DOT file)"
if [ -f "$OUTPUT_DIR/schema_$TIMESTAMP.png" ]; then
    echo "  - schema_$TIMESTAMP.png (ER diagram image)"
fi
echo ""
echo "Symlinks created:"
echo "  - schema_latest.* (links to latest files)"
