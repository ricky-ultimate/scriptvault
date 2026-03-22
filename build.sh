#!/usr/bin/env bash

set -e

BINARY_NAME="sv"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Cargo not found. Install Rust from https://rustup.rs/${NC}"
    exit 1
fi

BUILD_TYPE="debug"
RUN_TESTS=false
INSTALL=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --release) BUILD_TYPE="release"; shift ;;
        --test) RUN_TESTS=true; shift ;;
        --install) INSTALL=true; shift ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: ./build.sh [--release] [--test] [--install]"
            exit 1
            ;;
    esac
done

echo -e "${CYAN}Formatting...${NC}"
cargo fmt

echo -e "${CYAN}Linting...${NC}"
cargo clippy -- -D warnings

echo -e "${CYAN}Building ($BUILD_TYPE)...${NC}"
if [ "$BUILD_TYPE" = "release" ]; then
    cargo build --release
    BINARY_PATH="target/release/$BINARY_NAME"
else
    cargo build
    BINARY_PATH="target/debug/$BINARY_NAME"
fi

echo -e "${GREEN}Build successful${NC}"

if [ "$RUN_TESTS" = true ]; then
    echo -e "${CYAN}Testing...${NC}"
    cargo test
    echo -e "${GREEN}All tests passed${NC}"
fi

if [ -f "$BINARY_PATH" ]; then
    SIZE=$(du -h "$BINARY_PATH" | cut -f1)
    echo -e "${GREEN}Binary: $BINARY_PATH ($SIZE)${NC}"
    "$BINARY_PATH" --version
fi

if [ "$INSTALL" = true ]; then
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
    cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
    echo -e "${GREEN}Installed to $INSTALL_DIR/$BINARY_NAME${NC}"
    echo -e "${YELLOW}Ensure $INSTALL_DIR is in your PATH:${NC}"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

echo -e "${GREEN}Done${NC}"
