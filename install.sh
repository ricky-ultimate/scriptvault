#!/usr/bin/env bash
set -e

REPO="ricky-ultimate/scriptvault"
BINARY="sv"
INSTALL_DIR="${SV_INSTALL_DIR:-$HOME/.local/bin}"

# Detect OS and arch
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux)
    case "$ARCH" in
      x86_64)  TARGET="sv-linux-x86_64.tar.gz" ;;
      aarch64) TARGET="sv-linux-aarch64.tar.gz" ;;
      *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
    esac
    ;;
  darwin)
    case "$ARCH" in
      x86_64)  TARGET="sv-macos-x86_64.tar.gz" ;;
      arm64)   TARGET="sv-macos-aarch64.tar.gz" ;;
      *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS"
    echo "For Windows, use the PowerShell installer."
    exit 1
    ;;
esac

# Get latest release tag
echo "Fetching latest ScriptVault release..."
LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
  | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

if [ -z "$LATEST" ]; then
  echo "Failed to fetch latest release."
  exit 1
fi

echo "Installing ScriptVault $LATEST..."

URL="https://github.com/$REPO/releases/download/$LATEST/$TARGET"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

curl -fsSL "$URL" -o "$TMP/$TARGET"
tar -xzf "$TMP/$TARGET" -C "$TMP"

mkdir -p "$INSTALL_DIR"
mv "$TMP/$BINARY" "$INSTALL_DIR/$BINARY"
chmod +x "$INSTALL_DIR/$BINARY"

echo ""
echo "✓ ScriptVault $LATEST installed to $INSTALL_DIR/sv"
echo ""

# PATH reminder if needed
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
  echo "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
  echo ""
  echo '  export PATH="$HOME/.local/bin:$PATH"'
  echo ""
fi

"$INSTALL_DIR/$BINARY" --version
