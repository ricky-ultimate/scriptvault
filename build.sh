#!/usr/bin/env bash
# ScriptVault Build Script

set -e

PROJECT_NAME="scriptvault"
BINARY_NAME="sv"

echo "Building ScriptVault..."
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Check for cargo
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Cargo not found. Please install Rust from https://rustup.rs/${NC}"
    exit 1
fi

# Parse arguments
BUILD_TYPE="debug"
RUN_TESTS=false
INSTALL=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            BUILD_TYPE="release"
            shift
            ;;
        --test)
            RUN_TESTS=true
            shift
            ;;
        --install)
            INSTALL=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: ./build.sh [--release] [--test] [--install]"
            exit 1
            ;;
    esac
done

# Format code
echo -e "${CYAN} Formatting code...${NC}"
cargo fmt

# Run clippy
echo -e "${CYAN} Running clippy...${NC}"
cargo clippy -- -D warnings || {
    echo -e "${YELLOW} Clippy warnings found. Continuing anyway...${NC}"
}

# Build
echo -e "${CYAN} Building ($BUILD_TYPE)...${NC}"
if [ "$BUILD_TYPE" = "release" ]; then
    cargo build --release
    BINARY_PATH="target/release/$BINARY_NAME"
else
    cargo build
    BINARY_PATH="target/debug/$BINARY_NAME"
fi

echo -e "${GREEN}✓ Build successful!${NC}"

# Run tests
if [ "$RUN_TESTS" = true ]; then
    echo ""
    echo -e "${CYAN} Running tests...${NC}"
    cargo test
    echo -e "${GREEN} All tests passed!${NC}"
fi

# Check binary
if [ -f "$BINARY_PATH" ]; then
    echo ""
    echo -e "${GREEN}Binary created at: $BINARY_PATH${NC}"

    # Show size
    SIZE=$(du -h "$BINARY_PATH" | cut -f1)
    echo -e "${CYAN}Size: $SIZE${NC}"

    # Test binary
    echo ""
    echo -e "${CYAN}Testing binary...${NC}"
    "$BINARY_PATH" --version
fi

# Install
if [ "$INSTALL" = true ]; then
    echo ""
    echo -e "${CYAN} Installing...${NC}"

    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"

    cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"

    echo -e "${GREEN}✓ Installed to $INSTALL_DIR/$BINARY_NAME${NC}"
    echo ""
    echo -e "${YELLOW}Make sure $INSTALL_DIR is in your PATH:${NC}"
    echo -e "  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

echo ""
echo -e "${GREEN} Done!${NC}"
echo ""
echo "Quick start:"
echo "  ./$BINARY_PATH auth login --token local"
echo "  ./$BINARY_PATH save ./script.sh"
echo "  ./$BINARY_PATH list"
echo ""
