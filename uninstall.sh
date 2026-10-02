#!/usr/bin/env bash
# Uninstall m-ust (Linux/macOS):
#   curl -fsSL https://raw.githubusercontent.com/A2kliDis/m-ust/main/uninstall.sh | bash
# Pass --purge to also delete config + history:
#   curl -fsSL https://raw.githubusercontent.com/A2kliDis/m-ust/main/uninstall.sh | bash -s -- --purge
set -euo pipefail
NAME="m-ust"
EXE="${CARGO_HOME:-$HOME/.cargo}/bin/$NAME"
PURGE=0
[ "${1:-}" = "--purge" ] && PURGE=1

if pgrep -x "$NAME" >/dev/null 2>&1; then
  echo "$NAME is running. Quit it (q) first, then rerun this script." >&2
  exit 1
fi
if [ -f "$EXE" ]; then
  rm -f "$EXE"
  echo "Removed $EXE"
else
  echo "$NAME is not installed."
fi

case "$(uname -s)" in
  Darwin) DATADIR="$HOME/Library/Application Support/$NAME" ;;
  *) DATADIR="${XDG_DATA_HOME:-$HOME/.local/share}/$NAME" ;;
esac

if [ "$PURGE" = 1 ]; then
  rm -rf "$DATADIR"
  echo "Purged $DATADIR (config + history)."
else
  echo "Kept $DATADIR (config + history). Rerun with --purge to delete it."
fi
