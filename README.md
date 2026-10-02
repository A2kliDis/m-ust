# M-ust — Song recognizer TUI (100% Rust)

> Captures system audio or a specific app, recognizes the song via Shazam (free, no key).
>
> **Support:** Windows ✅ · Linux / macOS ⚠️ untested — reports welcome.
>
> English | [العربية](README_ar.md)

## Features
- Text UI (`ratatui`) — pick a source, press `r`
- Three sources: microphone, system audio (loopback), or a specific app (Windows)
- Optional AcoustID fallback with a free key
- Listening history (`h`) and continuous loop mode (`l` / `--loop-mode`)

Only anonymous fingerprints are uploaded — no raw audio ever leaves your machine.

## Install

```powershell
# One line (Windows) — prebuilt binary from the latest GitHub Release:
irm https://raw.githubusercontent.com/A2kliDis/m-ust/master/install.ps1 | iex
```

```bash
# One line (Linux/macOS):
curl -fsSL https://raw.githubusercontent.com/A2kliDis/m-ust/master/install.sh | bash
```

From source (requires Rust):

```powershell
cargo install --git https://github.com/A2kliDis/m-ust --bin m-ust
cargo install --path . --bin m-ust   # local checkout
```

Uninstall (asks about settings + history):

```powershell
irm https://raw.githubusercontent.com/A2kliDis/m-ust/master/uninstall.ps1 | iex
# curl -fsSL https://raw.githubusercontent.com/A2kliDis/m-ust/master/uninstall.sh | bash   # Linux/macOS
```

Installs to `%LOCALAPPDATA%\m-ust` (`~/.local/bin` on Linux/macOS) and adds it to PATH.
Portable mode (Windows, no PATH change): `$env:M_UST_NO_PATH=1; irm ... | iex`.

## Usage

```powershell
m-ust                                 # default 12 seconds
m-ust --duration 12
m-ust --loop-mode                     # keep listening until you quit
m-ust --acoustid-key YOUR_KEY         # enable AcoustID fallback
$env:M_UST_ACOUSTID_KEY="YOUR_KEY"; m-ust   # same, without saving to disk
```

Keys inside the TUI: `Tab` switch source • `↑/↓` select • `r` record • `l` loop • `h` history • `o` open song • `c` clear log • `q` quit

## Audio setup
- **Windows**: loopback works out of the box. Per-app capture needs Windows 10 2004+.
- **Linux**: pick a `Monitor of ...` source (`pactl list sources | grep monitor`).
- **macOS**: install BlackHole 2ch and select it as input.

## AcoustID fallback (optional)
1. Get a free client key at https://acoustid.org/new
2. Pass it via `--acoustid-key` or `M_UST_ACOUSTID_KEY`
3. Requires `fpcalc` (Chromaprint) next to `m-ust.exe` or in `PATH` — without it, Shazam-only mode works normally

Settings live in `%APPDATA%\m-ust\config.toml` (Windows) or `~/.config/m-ust/config.toml` (Linux) and are never committed.
