#!/bin/bash
# Download Gene Ontology data from Zenodo
# Usage: ./scripts/download_go_zenodo.sh

set -e

# Configuration
ZENODO_RECORD="17382285"
ZENODO_DOI="10.5281/zenodo.${ZENODO_RECORD}"
ARCHIVE_URL="https://zenodo.org/records/${ZENODO_RECORD}/files/go-release-archive.tgz"
DATA_DIR="data/go"
ARCHIVE_FILE="go-release-archive.tgz"

echo "=== Gene Ontology Zenodo Download Script ==="
echo ""
echo "Zenodo Record: ${ZENODO_RECORD}"
echo "DOI: ${ZENODO_DOI}"
echo "Data Directory: ${DATA_DIR}"
echo ""

# Create data directory
mkdir -p "${DATA_DIR}"

# Check if archive already exists
if [ -f "${DATA_DIR}/${ARCHIVE_FILE}" ]; then
    echo "Archive already exists. Skipping download."
    echo "Delete ${DATA_DIR}/${ARCHIVE_FILE} to re-download."
else
    echo "Downloading GO archive from Zenodo (21.4 GB)..."
    echo "This may take 10-30 minutes depending on your connection."
    echo ""

    # Download with wget (shows progress)
    if command -v wget &> /dev/null; then
        wget -O "${DATA_DIR}/${ARCHIVE_FILE}" "${ARCHIVE_URL}"
    elif command -v curl &> /dev/null; then
        curl -L -o "${DATA_DIR}/${ARCHIVE_FILE}" "${ARCHIVE_URL}"
    else
        echo "Error: Neither wget nor curl found. Please install one of them."
        exit 1
    fi

    echo ""
    echo "✓ Download complete"
fi

# Verify MD5 checksum (if available)
echo ""
echo "Verifying archive integrity..."
# Note: Add MD5 verification here if Zenodo provides checksums

# Extract go-basic.obo
echo ""
echo "Extracting go-basic.obo..."

if [ -f "${DATA_DIR}/go-basic.obo" ]; then
    echo "go-basic.obo already exists. Creating backup..."
    mv "${DATA_DIR}/go-basic.obo" "${DATA_DIR}/go-basic.obo.backup.$(date +%Y%m%d_%H%M%S)"
fi

# Extract the ontology file
tar -xzf "${DATA_DIR}/${ARCHIVE_FILE}" -C "${DATA_DIR}" --strip-components=2 \
    --wildcards '*/ontology/go-basic.obo' || {
    echo "Error: Failed to extract go-basic.obo"
    echo "The archive structure may have changed. Listing contents:"
    tar -tzf "${DATA_DIR}/${ARCHIVE_FILE}" | grep -i "go-basic.obo" | head -5
    exit 1
}

if [ -f "${DATA_DIR}/go-basic.obo" ]; then
    FILE_SIZE=$(stat -f%z "${DATA_DIR}/go-basic.obo" 2>/dev/null || stat -c%s "${DATA_DIR}/go-basic.obo" 2>/dev/null)
    FILE_SIZE_MB=$((FILE_SIZE / 1024 / 1024))

    echo "✓ Extraction complete"
    echo ""
    echo "=== Success! ==="
    echo ""
    echo "GO ontology file: ${DATA_DIR}/go-basic.obo (${FILE_SIZE_MB} MB)"
    echo ""
    echo "Configuration for BDP:"
    echo ""
    echo "  Environment variables:"
    echo "    export GO_LOCAL_ONTOLOGY_PATH=\"$(pwd)/${DATA_DIR}/go-basic.obo\""
    echo "    export GO_RELEASE_VERSION=\"2025-09-08\""
    echo "    export GO_ZENODO_DOI=\"${ZENODO_DOI}\""
    echo ""
    echo "  Or in Rust code:"
    echo "    let config = GoHttpConfig::zenodo_config("
    echo "        \"${DATA_DIR}/go-basic.obo\".to_string(),"
    echo "        \"2025-09-08\","
    echo "        \"${ZENODO_DOI}\","
    echo "    );"
    echo ""
    echo "Next steps:"
    echo "  1. Run: cargo run --bin go_test_human"
    echo "  2. See: docs/GO_INTEGRATION_GUIDE.md"
    echo ""
    echo "Attribution Notice:"
    echo "  Gene Ontology data from the 2025-09-08 release (DOI: ${ZENODO_DOI})"
    echo "  is made available under the terms of the CC BY 4.0 license."
else
    echo "Error: go-basic.obo not found after extraction"
    exit 1
fi

# Optional: Extract annotations if needed
read -p "Extract annotations as well? This will take several GB of space. [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Extracting annotations..."
    mkdir -p "${DATA_DIR}/annotations"
    tar -xzf "${DATA_DIR}/${ARCHIVE_FILE}" -C "${DATA_DIR}/annotations" \
        --strip-components=1 --wildcards '*/annotations/*'
    echo "✓ Annotations extracted to ${DATA_DIR}/annotations/"
fi

echo ""
echo "Done!"
