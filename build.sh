#!/usr/bin/env bash
#
# Build script for rmpc-theme-gen
#
# This builds the theme generator binary and optionally installs it
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

echo "Building rmpc-theme-gen..."
cd "$SCRIPT_DIR"

# Build release binary
cargo build --release

echo "✓ Build complete!"
echo "  Binary: $SCRIPT_DIR/target/release/rmpc-theme-gen"
echo "  Size: $(du -h "$SCRIPT_DIR/target/release/rmpc-theme-gen" | cut -f1)"

# Offer to install
if [ "${1:-}" = "install" ]; then
    echo ""
    echo "Installing to $INSTALL_DIR..."
    mkdir -p "$INSTALL_DIR"
    cp "$SCRIPT_DIR/target/release/rmpc-theme-gen" "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/rmpc-theme-gen"
    echo "✓ Installed to $INSTALL_DIR/rmpc-theme-gen"

    # Check if directory is in PATH
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        echo ""
        echo "⚠ Warning: $INSTALL_DIR is not in your PATH"
        echo "  Add this line to your ~/.bashrc or ~/.zshrc:"
        echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    fi
else
    echo ""
    echo "To install, run:"
    echo "  $0 install"
    echo ""
    echo "Or manually copy:"
    echo "  cp target/release/rmpc-theme-gen ~/.local/bin/"
fi