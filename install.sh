#!/usr/bin/env bash
# One-line install for m-ust (Linux/macOS, no Rust needed):
#   curl -fsSL https://raw.githubusercontent.com/A2kliDis/m-ust/master/install.sh | bash
# Custom folder (portable, no PATH handling):
#   curl -fsSL https://raw.githubusercontent.com/A2kliDis/m-ust/master/install.sh | bash -s -- --prefix "$HOME/apps/m-ust"
set -euo pipefail
REPO="A2kliDis/m-ust"
NAME="m-ust"
PREFIX="$HOME/.local/bin"
while [ $# -gt 0 ]; do
  case "$1" in
    --prefix=*) PREFIX="${1#--prefix=}"; shift ;;
    --prefix) PREFIX="${2:?--prefix needs a directory}"; shift 2 ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

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

mkdir -p "$PREFIX"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -fsSL --max-time 120 -o "$tmp/m-ust.tar.gz" "$url"
tar -xzf "$tmp/m-ust.tar.gz" -C "$tmp"
exe="$(find "$tmp" -name "$NAME" -type f | head -n1)"
[ -n "$exe" ] || { echo "archive contains no $NAME binary" >&2; exit 1; }
install -m 755 "$exe" "$PREFIX/$NAME"
echo "$NAME installed to $PREFIX"

if command -v "$NAME" >/dev/null 2>&1; then
  "$NAME" --version
else
  echo "Note: $PREFIX is not on PATH. Add this line to ~/.bashrc or ~/.zshrc:"
  echo "  export PATH=\"\$PATH:$PREFIX\""
fi
