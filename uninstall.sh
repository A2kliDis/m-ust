#!/usr/bin/env bash
# Uninstall m-ust (Linux/macOS):
#   curl -fsSL https://raw.githubusercontent.com/A2kliDis/m-ust/master/uninstall.sh | bash
# Pass --purge to also delete config + history.
set -euo pipefail
NAME="m-ust"
PURGE=0
if [ "${1:-}" = "--purge" ]; then PURGE=1; fi

if pgrep -x "$NAME" >/dev/null 2>&1; then
  echo "$NAME is running. Quit it (q) first, then rerun this script." >&2
  exit 1
fi
removed=0
# Current location + legacy cargo location (v0.1.0 installs)
for exe in "$HOME/.local/bin/$NAME" "${CARGO_HOME:-$HOME/.cargo}/bin/$NAME"; do
  if [ -f "$exe" ]; then
    rm -f "$exe"
    echo "Removed $exe"
    removed=1
  fi
done
[ "$removed" = 1 ] || echo "$NAME is not installed."

case "$(uname -s)" in
  Darwin) DATADIR="$HOME/Library/Application Support/$NAME" ;;
  *) DATADIR="${XDG_DATA_HOME:-$HOME/.local/share}/$NAME" ;;
esac
if [ "$PURGE" = 0 ]; then
  # Piped (curl | bash) has no stdin, so ask on the terminal directly if there is one.
  if [ -t 0 ]; then
    printf 'Also delete settings + history? [y/N] ' >&2
    read -r ans || true
    if [ "$ans" = "y" ] || [ "$ans" = "Y" ]; then PURGE=1; fi
  elif [ -e /dev/tty ]; then
    printf 'Also delete settings + history? [y/N] ' >/dev/tty 2>&1
    read -r ans </dev/tty || true
    if [ "$ans" = "y" ] || [ "$ans" = "Y" ]; then PURGE=1; fi
  fi
fi
if [ "$PURGE" = 1 ]; then
  rm -rf "$DATADIR"
  echo "Purged $DATADIR (config + history)."
else
  echo "Kept $DATADIR (config + history). Rerun with --purge to delete it."
fi
