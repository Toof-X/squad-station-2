#!/bin/sh
set -e

REPO="thientranhung/squad-station"
BASE_URL="https://github.com/thientranhung/squad-station/releases/download"
VERSION="0.5.1"

# Detect OS
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$OS" in
  darwin) ;;
  linux)  ;;
  *)
    echo "Unsupported OS: $OS" >&2
    echo "Please download manually from: https://github.com/${REPO}/releases/download/v${VERSION}/" >&2
    exit 1
    ;;
esac

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
  x86_64) ;;
  arm64|aarch64) ARCH="arm64" ;;
  *)
    echo "Unsupported architecture: $ARCH" >&2
    echo "Please download manually from: https://github.com/${REPO}/releases/download/v${VERSION}/" >&2
    exit 1
    ;;
esac

# Construct asset name and download URL
ASSET="squad-station-${OS}-${ARCH}"
URL="${BASE_URL}/v${VERSION}/${ASSET}"

# Determine install directory
INSTALL_DIR="/usr/local/bin"
FALLBACK=0
if [ ! -w "$INSTALL_DIR" ]; then
  INSTALL_DIR="$HOME/.local/bin"
  FALLBACK=1
  mkdir -p "$INSTALL_DIR"
fi

# Download to a temp file with cleanup on exit
TMPFILE=$(mktemp)
trap 'rm -f "$TMPFILE"' EXIT

echo "Downloading squad-station v${VERSION} (${OS}-${ARCH})..."
curl -fsSL --proto '=https' --tlsv1.2 -o "$TMPFILE" "$URL"

# Install and set permissions
mv "$TMPFILE" "${INSTALL_DIR}/squad-station"
chmod 755 "${INSTALL_DIR}/squad-station"

# Verify the binary is executable
if [ ! -x "${INSTALL_DIR}/squad-station" ]; then
  echo "Error: Installation failed — binary is not executable at ${INSTALL_DIR}/squad-station" >&2
  exit 1
fi

echo "Installed squad-station to ${INSTALL_DIR}/squad-station"
echo "Run: squad-station --version"

if [ "$FALLBACK" -eq 1 ]; then
  echo "Add ~/.local/bin to your PATH if not already present."
fi
