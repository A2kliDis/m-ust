//! Listening history — appended as CSV, never uploaded anywhere.
//! Location: `%APPDATA%/m-ust/history.csv` on Windows,
//! `~/.local/share/m-ust/history.csv` on Linux.

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
}

impl HistoryEntry {
    pub fn now(artist: String, title: String, album: Option<String>) -> Self {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        Self { timestamp, artist, title, album }
    }
}

pub fn history_path() -> PathBuf {
    if let Some(dir) = dirs::data_dir() {
        dir.join("m-ust").join("history.csv")
    } else {
        PathBuf::from("history.csv")
    }
}

const HEADER: &str = "timestamp,artist,title,album";

fn escape(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn parse_line(line: &str) -> Vec<String> {
    // Minimal CSV parser: handles quoted fields with "" escapes.
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;
    // Detect if line uses quoting
    let quoted = line.trim_start().starts_with('"');
    if !quoted {
        return line.split(',').map(|s| s.trim().to_string()).collect();
    }
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                cur.push(c);
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == ',' {
            fields.push(cur.clone());
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    fields.push(cur);
    fields
}

pub fn load() -> Vec<HistoryEntry> {
    let path = history_path();
    let txt = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    let mut out = Vec::new();
    for (i, line) in txt.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if i == 0 && line == HEADER {
            continue;
        }
        let f = parse_line(line);
        if f.len() < 3 {
            continue;
        }
        out.push(HistoryEntry {
            timestamp: f[0].clone(),
            artist: f[1].clone(),
            title: f[2].clone(),
            album: f.get(3).filter(|s| !s.is_empty()).cloned(),
        });
    }
    out
}

pub fn append(entry: &HistoryEntry) {
    let path = history_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let is_new = !path.exists();
    if is_new {
        let _ = std::fs::write(&path, format!("{}\n", HEADER));
    }
    let line = format!(
        "{},{},{},{}\n",
        escape(&entry.timestamp),
        escape(&entry.artist),
        escape(&entry.title),
        escape(entry.album.as_deref().unwrap_or(""))
    );
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}
