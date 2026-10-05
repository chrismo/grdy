#!/usr/bin/env bash
set -euo pipefail

REPO="chrismo/grdy"
BINARY="grdy"

# Determine install directory (XDG spec)
INSTALL_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"

# Parse arguments
VERSION="${1:-latest}"

# Detect OS
OS="$(uname -s)"
case "$OS" in
    # musl builds are static (any glibc); gnu is the fallback for releases before musl was published
    Linux)  OS_TARGETS="unknown-linux-musl unknown-linux-gnu" ;;
    Darwin) OS_TARGETS="apple-darwin" ;;
    *)      echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64)         ARCH_TARGET="x86_64" ;;
    aarch64|arm64)  ARCH_TARGET="aarch64" ;;
    *)              echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

# Get version tag
if [ "$VERSION" = "latest" ]; then
    VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
fi

if [ -z "$VERSION" ]; then
    echo "Failed to determine version" >&2
    exit 1
fi

# Accept both "0.7.0" and "v0.7.0"
VERSION="v${VERSION#v}"

WORK_DIR=$(mktemp -d)
trap 'rm -rf "$WORK_DIR"' EXIT

# Download the first available archive for this platform
ARCHIVE=""
for OS_TARGET in $OS_TARGETS; do
    TARGET="${ARCH_TARGET}-${OS_TARGET}"
    CANDIDATE="grdy-${VERSION}-${TARGET}.tar.gz"
    URL="https://github.com/${REPO}/releases/download/${VERSION}/${CANDIDATE}"
    echo "Downloading $URL..."
    if curl -fsL "$URL" -o "$WORK_DIR/$CANDIDATE"; then
        ARCHIVE="$CANDIDATE"
        break
    fi
done

if [ -z "$ARCHIVE" ]; then
    echo "No $BINARY $VERSION release found for ${ARCH_TARGET} ${OS}" >&2
    exit 1
fi

echo "Installing $BINARY $VERSION for $TARGET..."
tar xzf "$WORK_DIR/$ARCHIVE" -C "$WORK_DIR"

mkdir -p "$INSTALL_DIR"
mv "$WORK_DIR/$BINARY" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/$BINARY"

echo "Installed $BINARY to $INSTALL_DIR/$BINARY"

# Check if in PATH
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    echo ""
    echo "Note: $INSTALL_DIR is not in your PATH."
    echo "Add it with: export PATH=\"$INSTALL_DIR:\$PATH\""
fi
