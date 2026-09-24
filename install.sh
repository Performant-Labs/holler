#!/bin/sh
# Install the latest (or a pinned) `holler` release binary.
#
#   curl -fsSL https://raw.githubusercontent.com/Performant-Labs/holler/main/install.sh | sh
#
# Env overrides:
#   HOLLER_VERSION   release tag to install, e.g. v0.3.0 (default: latest)
#   HOLLER_INSTALL_DIR   where to place the binary (default: $HOME/.local/bin)
#
# Release assets today: macos-latest (Apple Silicon), ubuntu-latest
# (x86_64 Linux), and holler-ubuntu-arm64 (aarch64 Linux, built on GitHub's
# hosted ubuntu-24.04-arm runner — see .github/workflows/release-arm64.yml).
# See https://github.com/Performant-Labs/holler/releases. Windows is not a
# supported target (issue #378: the control-socket transport is Unix domain
# sockets end to end).

set -eu

REPO="Performant-Labs/holler"
VERSION="${HOLLER_VERSION:-latest}"
INSTALL_DIR="${HOLLER_INSTALL_DIR:-$HOME/.local/bin}"

say() { printf '%s\n' "$*" >&2; }
die() { say "error: $*"; exit 1; }

os="$(uname -s)"
case "$os" in
  Darwin) asset="holler-macos-latest" ;;
  Linux)
    arch="$(uname -m)"
    case "$arch" in
      x86_64|amd64) asset="holler-ubuntu-latest" ;;
      aarch64|arm64) asset="holler-ubuntu-arm64" ;;
      *) die "unsupported Linux architecture '$arch' — published Linux binaries are x86_64 and arm64 only; build from source instead (cargo build --release -p holler-cli)" ;;
    esac
    ;;
  *) die "unsupported OS '$os' — only macOS and Linux release binaries exist (issue #378 tracks Windows)" ;;
esac

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
