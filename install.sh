#!/bin/sh
set -eu

REPO="ppdx999/mdd"
INSTALL_DIR="${MDD_INSTALL_DIR:-$HOME/.local/bin}"

# Detect OS
OS="$(uname -s)"
case "$OS" in
  Linux)  os="unknown-linux-gnu" ;;
  Darwin) os="apple-darwin" ;;
  *)      echo "Unsupported OS: $OS"; exit 1 ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64)  arch="x86_64" ;;
  aarch64|arm64) arch="aarch64" ;;
  *)             echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

TARGET="${arch}-${os}"

# Get latest version if not specified
if [ -z "${MDD_VERSION:-}" ]; then
  MDD_VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
fi

if [ -z "$MDD_VERSION" ]; then
  echo "Failed to determine latest version"
  exit 1
fi

ARCHIVE="mdd-${MDD_VERSION}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${MDD_VERSION}/${ARCHIVE}"

echo "Installing mdd ${MDD_VERSION} for ${TARGET}..."
echo "  from: ${URL}"
echo "  to:   ${INSTALL_DIR}"

# Download and extract
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

curl -fsSL "$URL" -o "$TMPDIR/$ARCHIVE"
tar xzf "$TMPDIR/$ARCHIVE" -C "$TMPDIR"

# Install binaries
mkdir -p "$INSTALL_DIR"
for bin in "$TMPDIR/mdd-${MDD_VERSION}-${TARGET}"/mdd*; do
  if [ -f "$bin" ] && [ -x "$bin" ]; then
    cp "$bin" "$INSTALL_DIR/"
    echo "  installed: $(basename "$bin")"
  fi
done

echo ""
echo "Done! Make sure ${INSTALL_DIR} is in your PATH."
echo ""
echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
echo ""
