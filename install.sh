#!/usr/bin/env bash
# One-line install for m-ust (Linux/macOS):
#   curl -fsSL https://raw.githubusercontent.com/A2kliDis/m-ust/main/install.sh | bash
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

release_json="$(curl -fsSL --max-time 20 "https://api.github.com/repos/$REPO/releases/latest")"
url="$(printf '%s' "$release_json" \
  | grep -o "\"browser_download_url\": *\"[^\"]*${want}[^\"]*\"" \
  | head -n1 | cut -d'"' -f4)"
[ -n "$url" ] || { echo "No $want binary in the latest release" >&2; exit 1; }

mkdir -p "$BINDIR"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -fsSL --max-time 120 -o "$tmp/m-ust.tar.gz" "$url"
tar -xzf "$tmp/m-ust.tar.gz" -C "$tmp"
exe="$(find "$tmp" -name "$NAME" -type f | head -n1)"
[ -n "$exe" ] || { echo "archive contains no $NAME binary" >&2; exit 1; }
install -m 755 "$exe" "$BINDIR/$NAME"
echo "$NAME installed to $BINDIR"
"$BINDIR/$NAME" --version
