#!/bin/sh
# Install the latest (or a pinned) `holler` release binary.
#
#   curl -fsSL https://raw.githubusercontent.com/Performant-Labs/holler/main/install.sh | sh
#
# Env overrides:
#   HOLLER_VERSION   release tag to install, e.g. v0.2.0 (default: latest)
#   HOLLER_INSTALL_DIR   where to place the binary (default: $HOME/.local/bin)
#
# Only ubuntu-latest (x86_64 Linux) and macos-latest (Apple Silicon) release
# assets exist today — see https://github.com/Performant-Labs/holler/releases.
# Windows is not a supported target (issue #378: the control-socket transport
# is Unix domain sockets end to end).

set -eu

REPO="Performant-Labs/holler"
VERSION="${HOLLER_VERSION:-latest}"
INSTALL_DIR="${HOLLER_INSTALL_DIR:-$HOME/.local/bin}"

say() { printf '%s\n' "$*" >&2; }
die() { say "error: $*"; exit 1; }

os="$(uname -s)"
case "$os" in
  Darwin) asset="holler-macos-latest" ;;
  Linux)  asset="holler-ubuntu-latest" ;;
  *) die "unsupported OS '$os' — only macOS and Linux release binaries exist (issue #378 tracks Windows)" ;;
esac

if [ "$os" = "Linux" ]; then
  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64) : ;;
    *) die "unsupported Linux architecture '$arch' — the published Linux binary is x86_64 only; build from source instead (cargo build --release -p holler-cli)" ;;
  esac
fi

if [ "$VERSION" = "latest" ]; then
  url="https://github.com/$REPO/releases/latest/download/$asset"
else
  url="https://github.com/$REPO/releases/download/$VERSION/$asset"
fi

command -v curl >/dev/null 2>&1 || die "curl is required"

mkdir -p "$INSTALL_DIR"
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

say "Downloading $asset ($VERSION) from $REPO..."
http_code="$(curl -fsSL -w '%{http_code}' -o "$tmp" "$url" || true)"
[ "$http_code" = "200" ] || die "download failed (HTTP $http_code) — $url"

chmod +x "$tmp"
mv "$tmp" "$INSTALL_DIR/holler"
trap - EXIT

say "Installed to $INSTALL_DIR/holler"

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) say "Note: $INSTALL_DIR is not on your PATH. Add this to your shell profile:"
     say "  export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
esac

"$INSTALL_DIR/holler" --version >&2 || die "installed binary failed to run — see above"
