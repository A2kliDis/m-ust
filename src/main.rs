//! m-ust - Song recognizer TUI
//! 100% Rust + 100% free (Shazam fingerprint + AcoustID fallback)
//! Supports: System Audio (Loopback) + Per-App capture (Windows Process Loopback)

mod api;
mod audio;
mod config;
mod fingerprint;
mod history;
mod tui;

use anyhow::Result;
use clap::Parser;
use config::AppConfig;

#[derive(Parser, Debug)]
#[command(name="m-ust", version, about="Song recognizer TUI (100% Rust)")]
struct Args {
    /// Recording duration in seconds [default: 12]
    #[arg(short, long)]
    duration: Option<u64>,

    /// AcoustID API key (optional, for free fallback).
    /// Also read from env M_UST_ACOUSTID_KEY (or ACOUSTID_KEY). CLI wins over env, env wins over config file.
    #[arg(long)]
    acoustid_key: Option<String>,

    /// Start in continuous listening mode (like `songrec listen`)
    #[arg(long)]
    loop_mode: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();
    let mut config = AppConfig::load();
    // None = flag absent → keep config file (or built-in 12s) value
    config.merge_args(args.duration, args.acoustid_key);

    let local = tokio::task::LocalSet::new();
    local.run_until(tui::run(config, args.loop_mode)).await
}
