#!/usr/bin/env bash
# Build the release binary without embedding the builder's home directory.
#
# A plain `cargo build --release` bakes absolute source and registry paths (panic
# locations, debug maps) into the binary, so the builder's OS username and home
# directory end up in a published artifact. This wraps the build with path
# remapping and symbol stripping, then refuses to succeed if a personal path is
# still visible in the result.
#
# Usage (from the workspace root, on a checkout of the release tag):
#   scripts/release-build.sh
# Output: target/release/holler
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

cargo_home="${CARGO_HOME:-$HOME/.cargo}"
rustup_home="${RUSTUP_HOME:-$HOME/.rustup}"

# rustc applies the last matching mapping, so list the broadest ($HOME) last.
flags="--remap-path-prefix=$root=/build/src"
flags="$flags --remap-path-prefix=$cargo_home=/build/cargo"
flags="$flags --remap-path-prefix=$rustup_home=/build/rustup"
flags="$flags --remap-path-prefix=$HOME=/build/home"

RUSTFLAGS="${RUSTFLAGS:-} $flags" CARGO_PROFILE_RELEASE_STRIP=symbols \
  cargo build --release -p holler-cli

"$root/scripts/check-binary-paths.sh" "$root/target/release/holler"
