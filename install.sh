#!/usr/bin/env bash
# One-line install for m-ust (Linux/macOS):
#   curl -fsSL https://raw.githubusercontent.com/A2kliDis/m-ust/main/install.sh | bash
#
# Tries a prebuilt binary from the latest GitHub Release first,
# falls back to `cargo install --git` (needs Rust + repo access).
set -euo pipefail
REPO="A2kliDis/m-ust"
NAME="m-ust"
BINDIR="${CARGO_HOME:-$HOME/.cargo}/bin"

os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Linux) plat="linux" ;;
  Darwin) plat="macos" ;;
  *) echo "Unsupported OS: $os" >&2; exit 1 ;;
esac
case "$arch" in
  x86_64|amd64) arch="x86_64" ;;
  arm64|aarch64) arch="aarch64" ;;
  *) echo "Unsupported arch: $arch" >&2; exit 1 ;;
esac
want="${plat}-${arch}"

install_via_cargo() {
  command -v cargo >/dev/null || { echo "cargo not found. Install Rust from https://rustup.rs/ first." >&2; exit 1; }
  cargo install --git "https://github.com/$REPO" --bin "$NAME" --locked
}

url=""
if release_json=$(curl -fsSL --max-time 20 "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null); then
  url=$(printf '%s' "$release_json" \
    | grep -o "\"browser_download_url\": *\"[^\"]*${want}[^\"]*\"" \
    | head -n1 | cut -d'"' -f4 || true)
fi

if [ -n "$url" ]; then
  mkdir -p "$BINDIR"
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  curl -fsSL --max-time 120 -o "$tmp/m-ust.tar.gz" "$url"
  tar -xzf "$tmp/m-ust.tar.gz" -C "$tmp"
  exe="$(find "$tmp" -name "$NAME" -type f | head -n1)"
  [ -n "$exe" ] || { echo "archive contains no $NAME binary" >&2; exit 1; }
  install -m 755 "$exe" "$BINDIR/$NAME"
  echo "$NAME installed to $BINDIR"
else
  echo "Prebuilt download failed; falling back to 'cargo install --git'..." >&2
  install_via_cargo
fi

"$BINDIR/$NAME" --version
