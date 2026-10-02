use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub record_duration_secs: u64,
    pub acoustid_api_key: Option<String>,
    pub default_input: Option<String>,
    pub default_output: Option<String>,
    pub default_tab: Option<usize>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            record_duration_secs: 12,
            acoustid_api_key: None,
            default_input: None,
            default_output: None,
            default_tab: None,
        }
    }
}

fn config_path() -> PathBuf {
    if let Some(dir) = dirs::config_dir() {
        dir.join("m-ust").join("config.toml")
    } else {
        PathBuf::from("config.toml")
    }
}

/// Env vars checked for the AcoustID key (in priority order).
/// CLI flag `--acoustid-key` still wins over all of these (see `merge_args`).
pub const ACOUSTID_ENV_VARS: [&str; 2] = ["M_UST_ACOUSTID_KEY", "ACOUSTID_KEY"];

fn acoustid_key_from_env() -> Option<String> {
    for var in ACOUSTID_ENV_VARS {
        if let Ok(v) = std::env::var(var) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

impl AppConfig {
    pub fn load() -> Self {
        let mut cfg = Self::load_from_file();
        // Env overrides file (CLI flag overrides env later in merge_args).
        if let Some(k) = acoustid_key_from_env() {
            cfg.acoustid_api_key = Some(k);
        }
        cfg
    }

    fn load_from_file() -> Self {
        let path = config_path();
        if let Ok(txt) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = toml::from_str::<AppConfig>(&txt) {
                return cfg;
            }
        }
        // Try local dir fallback
        if let Ok(txt) = std::fs::read_to_string("config.toml") {
            if let Ok(cfg) = toml::from_str::<AppConfig>(&txt) {
                return cfg;
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Never persist a key that came from the environment into the
        // plaintext file — keep env secrets transient.
        let mut to_save = self.clone();
        if let Some(env_key) = acoustid_key_from_env() {
            if to_save.acoustid_api_key.as_ref() == Some(&env_key) {
                to_save.acoustid_api_key = None;
            }
        }
        let txt = toml::to_string_pretty(&to_save)?;
        std::fs::write(&path, &txt)?;
        Ok(())
    }

    pub fn merge_args(&mut self, duration: Option<u64>, acoustid_key: Option<String>) {
        if let Some(d) = duration { self.record_duration_secs = d; }
        if let Some(k) = acoustid_key { self.acoustid_api_key = Some(k); }
    }
}
