#!/bin/sh
# BDP Universal Installer
#
# This script installs the BDP CLI from GitHub releases.
# It supports multiple channels (stable, canary, specific versions)
# and performs checksum verification for security.
#
# Usage:
#   curl -fsSL https://install.bdp.dev/install.sh | sh
#   curl -fsSL https://install.bdp.dev/install.sh | sh -s -- --channel canary
#   curl -fsSL https://install.bdp.dev/install.sh | sh -s -- --version v0.1.0
#   curl -fsSL https://install.bdp.dev/install.sh | sh -s -- --path /usr/local/bin

set -e

# Configuration
REPO="datadir-lab/bdp"
CHANNEL="stable"
VERSION=""
INSTALL_PATH="${CARGO_HOME:-$HOME/.cargo}/bin"
BINARY_NAME="bdp"
GITHUB_API="https://api.github.com"
GITHUB_DOWNLOAD="https://github.com"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
info() {
    printf "${BLUE}==>${NC} %s\n" "$1"
}

success() {
    printf "${GREEN}==>${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}Warning:${NC} %s\n" "$1"
}

error() {
    printf "${RED}Error:${NC} %s\n" "$1" >&2
    exit 1
}

# Parse command line arguments
parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            --channel)
                CHANNEL="$2"
                shift 2
                ;;
            --version)
                VERSION="$2"
                CHANNEL="specific"
                shift 2
                ;;
            --path)
                INSTALL_PATH="$2"
                shift 2
                ;;
            --help)
                cat <<EOF
BDP Universal Installer

USAGE:
    install.sh [OPTIONS]

OPTIONS:
    --channel <CHANNEL>    Install from channel: stable (default), canary
    --version <VERSION>    Install specific version (e.g., v0.1.0)
    --path <PATH>          Install to custom path (default: \$CARGO_HOME/bin or ~/.cargo/bin)
    --help                 Show this help message

EXAMPLES:
    # Install latest stable release
    curl -fsSL https://install.bdp.dev/install.sh | sh

    # Install canary release
    curl -fsSL https://install.bdp.dev/install.sh | sh -s -- --channel canary

    # Install specific version
    curl -fsSL https://install.bdp.dev/install.sh | sh -s -- --version v0.1.0

    # Install to custom path
    curl -fsSL https://install.bdp.dev/install.sh | sh -s -- --path /usr/local/bin
EOF
                exit 0
                ;;
            *)
                error "Unknown option: $1. Use --help for usage information."
                ;;
        esac
    done
}

# Detect platform
detect_platform() {
    local os arch platform

    # Detect OS
    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    case "$os" in
        linux)
            os="unknown-linux-gnu"
            ;;
        darwin)
            os="apple-darwin"
            ;;
        msys*|mingw*|cygwin*)
            error "Windows detected. Please use install.ps1 instead: iwr https://install.bdp.dev/install.ps1 -useb | iex"
            ;;
        *)
            error "Unsupported operating system: $os"
            ;;
    esac

    # Detect architecture
    arch=$(uname -m)
    case "$arch" in
        x86_64|amd64)
            arch="x86_64"
            ;;
        aarch64|arm64)
            arch="aarch64"
            ;;
        *)
            error "Unsupported architecture: $arch"
            ;;
    esac

    platform="${arch}-${os}"
    echo "$platform"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Download helper (tries curl, falls back to wget)
download() {
    local url="$1"
    local output="$2"

    if command_exists curl; then
        curl --proto '=https' --tlsv1.2 -fsSL -o "$output" "$url"
    elif command_exists wget; then
        wget --https-only -q -O "$output" "$url"
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

# Query GitHub API
github_api_get() {
    local endpoint="$1"
    local url="${GITHUB_API}${endpoint}"
    local temp_file
    temp_file=$(mktemp)

    if command_exists curl; then
        curl --proto '=https' --tlsv1.2 -fsSL "$url" > "$temp_file"
    elif command_exists wget; then
        wget --https-only -q -O "$temp_file" "$url"
    else
        error "Neither curl nor wget found. Please install one of them."
    fi

    cat "$temp_file"
    rm -f "$temp_file"
}

# Resolve version based on channel
resolve_version() {
    info "Resolving version for channel: $CHANNEL"

    case "$CHANNEL" in
        stable)
            # Get latest non-prerelease
            local latest
            latest=$(github_api_get "/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/')
            if [ -z "$latest" ]; then
                error "Failed to resolve latest stable version"
            fi
            VERSION="$latest"
            ;;
        canary)
            # Get latest prerelease
            local releases
            releases=$(github_api_get "/repos/${REPO}/releases")
            local canary
            canary=$(echo "$releases" | grep -B 3 '"prerelease": true' | grep '"tag_name":' | head -n 1 | sed -E 's/.*"tag_name": "([^"]+)".*/\1/')
            if [ -z "$canary" ]; then
                warn "No canary releases found, falling back to latest stable"
                VERSION=$(echo "$releases" | grep '"tag_name":' | head -n 1 | sed -E 's/.*"tag_name": "([^"]+)".*/\1/')
            else
                VERSION="$canary"
            fi
            ;;
        specific)
            # Version already set via --version flag
            if [ -z "$VERSION" ]; then
                error "Version not specified for specific channel"
            fi
            # Ensure version starts with 'v'
            if [ "${VERSION#v}" = "$VERSION" ]; then
                VERSION="v${VERSION}"
            fi
            ;;
        *)
            error "Unknown channel: $CHANNEL"
            ;;
    esac

    info "Resolved version: $VERSION"
}

# Download binary and checksum
download_binary() {
    local platform="$1"
    local archive_name="${BINARY_NAME}-${platform}.tar.gz"
    local download_url="${GITHUB_DOWNLOAD}/${REPO}/releases/download/${VERSION}/${archive_name}"
    local checksum_url="${download_url}.sha256"
    local temp_dir
    temp_dir=$(mktemp -d)

    info "Downloading $BINARY_NAME $VERSION for $platform..."

    # Download archive
    download "$download_url" "$temp_dir/$archive_name" || {
        rm -rf "$temp_dir"
        error "Failed to download binary from $download_url"
    }

    # Download checksum
    download "$checksum_url" "$temp_dir/$archive_name.sha256" || {
        warn "Failed to download checksum file, skipping verification"
        echo "$temp_dir"
        return
    }

    echo "$temp_dir"
}

# Verify checksum
verify_checksum() {
    local temp_dir="$1"
    local platform="$2"
    local archive_name="${BINARY_NAME}-${platform}.tar.gz"
    local archive_path="$temp_dir/$archive_name"
    local checksum_path="$temp_dir/$archive_name.sha256"

    if [ ! -f "$checksum_path" ]; then
        warn "Checksum file not found, skipping verification"
        return 0
    fi

    info "Verifying checksum..."

    # Read expected checksum (format: "checksum  filename" or just "checksum")
    local expected_checksum
    expected_checksum=$(cat "$checksum_path" | awk '{print $1}')

    # Calculate actual checksum
    local actual_checksum
    if command_exists sha256sum; then
        actual_checksum=$(sha256sum "$archive_path" | awk '{print $1}')
    elif command_exists shasum; then
        actual_checksum=$(shasum -a 256 "$archive_path" | awk '{print $1}')
    else
        warn "sha256sum or shasum not found, skipping checksum verification"
        return 0
    fi

    if [ "$expected_checksum" != "$actual_checksum" ]; then
        error "Checksum verification failed!\nExpected: $expected_checksum\nActual:   $actual_checksum"
    fi

    success "Checksum verified"
}

# Extract and install binary
install_binary() {
    local temp_dir="$1"
    local platform="$2"
    local archive_name="${BINARY_NAME}-${platform}.tar.gz"
    local archive_path="$temp_dir/$archive_name"

    info "Extracting binary..."

    # Extract archive
    tar -xzf "$archive_path" -C "$temp_dir" || error "Failed to extract archive"

    # Create install directory if it doesn't exist
    if [ ! -d "$INSTALL_PATH" ]; then
        info "Creating install directory: $INSTALL_PATH"
        mkdir -p "$INSTALL_PATH" || error "Failed to create install directory: $INSTALL_PATH"
    fi

    # Find the binary in the extracted files
    local binary_path
    binary_path=$(find "$temp_dir" -type f -name "$BINARY_NAME" | head -n 1)

    if [ -z "$binary_path" ]; then
        error "Binary not found in archive"
    fi

    # Install binary
    info "Installing to $INSTALL_PATH/$BINARY_NAME"

    # Remove old binary if it exists
    if [ -f "$INSTALL_PATH/$BINARY_NAME" ]; then
        rm -f "$INSTALL_PATH/$BINARY_NAME"
    fi

    cp "$binary_path" "$INSTALL_PATH/$BINARY_NAME" || error "Failed to copy binary"
    chmod +x "$INSTALL_PATH/$BINARY_NAME" || error "Failed to make binary executable"

    # Cleanup
    rm -rf "$temp_dir"

    success "Installation complete!"
}

# Verify installation
verify_installation() {
    info "Verifying installation..."

    # Check if binary is in PATH
    if ! command_exists "$BINARY_NAME"; then
        warn "$BINARY_NAME is not in your PATH"
        warn "Add the following to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo ""
        echo "    export PATH=\"$INSTALL_PATH:\$PATH\""
        echo ""
    fi

    # Try to run the binary directly
    local version_output
    if version_output=$("$INSTALL_PATH/$BINARY_NAME" --version 2>&1); then
        success "Installed: $version_output"
    else
        error "Installation verification failed"
    fi
}

# Main installation flow
main() {
    parse_args "$@"

    info "BDP Universal Installer"
    info "========================"

    local platform
    platform=$(detect_platform)
    info "Detected platform: $platform"

    resolve_version

    local temp_dir
    temp_dir=$(download_binary "$platform")

    verify_checksum "$temp_dir" "$platform"

    install_binary "$temp_dir" "$platform"

    verify_installation

    echo ""
    success "BDP $VERSION installed successfully!"
    echo ""
    info "Get started by running:"
    echo "    $BINARY_NAME --help"
    echo ""
}

# Run main function with all arguments
main "$@"
