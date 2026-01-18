#!/bin/sh
# BDP Uninstall Script
# This script uninstalls the BDP CLI tool from your system

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "${GREEN}BDP Uninstall Script${NC}"
echo "===================="
echo ""

# Determine install location
if [ -n "$CARGO_HOME" ]; then
    INSTALL_DIR="$CARGO_HOME/bin"
else
    INSTALL_DIR="$HOME/.cargo/bin"
fi

BDP_PATH="$INSTALL_DIR/bdp"

# Check if BDP is installed
if [ ! -f "$BDP_PATH" ]; then
    echo "${YELLOW}BDP is not installed at $BDP_PATH${NC}"
    echo "Nothing to uninstall."
    exit 0
fi

# Confirm uninstallation
printf "${YELLOW}This will remove BDP from: $BDP_PATH${NC}\n"
printf "Continue? (y/N): "
read -r CONFIRM

case "$CONFIRM" in
    [yY]|[yY][eE][sS])
        echo "Proceeding with uninstallation..."
        ;;
    *)
        echo "Uninstallation cancelled."
        exit 0
        ;;
esac

# Remove the binary
echo "Removing BDP binary..."
rm -f "$BDP_PATH"

# Remove cache directory (optional, ask user)
CACHE_DIR="$HOME/.cache/bdp"
if [ -d "$CACHE_DIR" ]; then
    printf "${YELLOW}Remove BDP cache directory? ($CACHE_DIR)${NC} (y/N): "
    read -r CONFIRM_CACHE

    case "$CONFIRM_CACHE" in
        [yY]|[yY][eE][sS])
            echo "Removing cache directory..."
            rm -rf "$CACHE_DIR"
            ;;
        *)
            echo "Keeping cache directory."
            ;;
    esac
fi

echo ""
echo "${GREEN}✓ BDP has been successfully uninstalled!${NC}"
echo ""
echo "To reinstall BDP in the future, visit:"
echo "  https://github.com/datadir-lab/bdp"
