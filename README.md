# M-ust — Song recognizer TUI (100% Rust)

> Fully built in Rust • 100% free • captures system audio or a specific app
>
> English | [العربية](README_ar.md)

## Features
- **ratatui TUI**: fast text UI — pick a source and press `r`
- **Three capture modes**:
  1. 🎤 Microphone (CPAL)
  2. 🔊 System audio loopback — everything coming out of the speakers
  3. 🎯 Specific app (Windows process loopback)
- **Pure-Rust Shazam fingerprint**: `rustfft` + Wang 2003 algorithm (like SongRec) — no C libraries
- **100% free**: sends only the fingerprint (peaks) to `amp.shazam.com` — no key needed, no raw audio uploaded
- **AcoustID fallback**: optional if Shazam fails, with a free key from `acoustid.org`

## How it works

```
[ CPAL / WASAPI Loopback ] -> mono 16kHz (downmix+resample)
        ↓
[ Shazam Fingerprint ] 2048 FFT + Hanning -> Peak Spreading (freq/time) -> 4 bands
        ↓
[ POST https://amp.shazam.com/discovery/v5/... ] -> JSON {title, artist, album, url}
```

The fingerprint is just a list of spectral peaks `(freq, time)` — audio cannot be reconstructed from it. Privacy preserved.

## Install

```powershell
# From source (requires Rust)
cargo install --git https://github.com/A2kliDis/m-ust --bin m-ust

# Or locally
cargo install --path . --bin m-ust
```

## Usage

```powershell
m-ust                    # default 12 seconds
m-ust --duration 12
m-ust --acoustid-key YOUR_KEY --duration 15
# Or via env var (never written to the config file):
$env:M_UST_ACOUSTID_KEY="YOUR_KEY"; m-ust
```

For development:

```powershell
cargo run -- --duration 12
cargo run -- --acoustid-key YOUR_KEY --duration 15
```

Inside the TUI:
- `Tab`: switch source (mic / device / app)
- `↑/↓`: select device/app
- `r`: record and recognize
- `o`: open cover art (or song link) in the browser
- `l`: continuous listening Loop ON/OFF (or `m-ust --loop-mode`) — skips consecutive repeats of the same song automatically
- `h`: show last 5 of history (auto-saved to `%APPDATA%\m-ust\history.csv`)
- `c`: clear log
- `q`: quit

## System audio capture — per OS

### Windows (tested)
- **System Loopback**: built in via WASAPI loopback — captures exactly what you hear. If it fails, try another output device, or enable `Stereo Mix` in sound settings as a fallback.
- **Per-App**: via Windows process loopback (requires Windows 10 2004+). Pick `chrome.exe` / `spotify.exe` from the list; only that process is captured (real PID via `IAudioSessionManager2`).

### Linux
- **System Loopback**: look for a `Monitor of ...` device (PulseAudio/PipeWire). The code auto-detects any input containing `monitor`.
  ```bash
  pactl list sources | grep monitor
  # then select it in the TUI
  ```
  With PipeWire + Pulse together you may see `no node available` — remove `pulseaudio` and install `pipewire-pulse`.

### macOS
- No built-in loopback. Install **BlackHole 2ch**:
  ```bash
  brew install blackhole-2ch
  ```
  Then create a Multi-Output Device in `Audio MIDI Setup`, select it as output, and `BlackHole` as input in the TUI.

## 100% free — what we used

| Component | License | Cost |
|-----------|---------|------|
| `ratatui`, `crossterm`, `cpal`, `rustfft`, `reqwest`, `hound` | MIT/Apache2 | free |
| `wasapi` (Windows) | MIT | free |
| Shazam `amp.shazam.com` (unofficial, as used by SongRec) | free, no key | free |
| AcoustID `api.acoustid.org` | free with free key | free (3 req/s) |
| MusicBrainz | free | free |

**No AudD, no ACRCloud, no paid keys.**

Getting a free AcoustID key (optional):
1. Go to https://acoustid.org/new
2. Register and get a `Client API Key`
3. Run `m-ust --acoustid-key KEY` or set the `M_UST_ACOUSTID_KEY` env var

> Note: the AcoustID path is fallback-only and requires `fpcalc` (from Chromaprint).
> Without it the tool works normally via Shazam. Do not commit `fpcalc.exe` to the repo —
> place it next to `m-ust.exe` or in `PATH`.
> The config file (`%APPDATA%\m-ust\config.toml`) is never committed (in `.gitignore`).

## Project structure

```
src/
  main.rs              # CLI + TUI entry
  config.rs            # AppConfig (env > file)
  history.rs           # history.csv save/load
  audio/
    capture.rs         # CPAL + WASAPI loopback + process loopback + resample 16k
    devices.rs         # device + real app-session enumeration
  fingerprint/
    shazam.rs          # SignatureGenerator (port of SongRec)
  api/
    shazam.rs          # POST to Shazam
    acoustid.rs        # fallback via fpcalc
  tui/
    app.rs             # app state, loop, continuous listening
    ui.rs              # ratatui rendering
```

## Roadmap
- [x] Real WASAPI loopback (`src/audio/capture.rs`)
- [x] Real PIDs via `IAudioSessionManager2` (no fake names)
- [x] History in `history.csv` (view with `h`)
- [x] `fpcalc` integration for real AcoustID (optional, requires `chromaprint`)
- [x] Continuous listening (`l` or `m-ust --loop-mode`)
- [ ] In-app update notification (check GitHub Releases)

## Release build

```powershell
cargo build --release
.\target\release\m-ust.exe
```

> Built with Rust 1.98, runs on Windows/Linux/macOS.
