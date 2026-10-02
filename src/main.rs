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
    /// Recording duration in seconds
    #[arg(short, long, default_value_t = 12)]
    duration: u64,

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
    // CLI args override file (only if user explicitly passed --duration, but clap always gives 12)
    // So we check if args differ from default via raw parsing: for now, if --duration was passed, it will be !=12 only if user changed
    // Simpler: always use CLI duration if provided, but config file is default 12 so it's fine to override
    let duration_opt = if args.duration != 12 { Some(args.duration) } else { None };
    config.merge_args(duration_opt, args.acoustid_key);

    let local = tokio::task::LocalSet::new();
    local.run_until(tui::run(config, args.loop_mode)).await
}
