use anyhow::{Result, anyhow, Context};
use serde_json::Value;
use std::process::Command;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AcoustIdResult {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
}

pub async fn recognize_with_acoustid(samples: &[i16], sample_rate: u32, api_key: &str) -> Result<AcoustIdResult> {
    // 1. Save to temp wav
    let mut tmp = std::env::temp_dir();
    tmp.push(format!("must_acoustid_{}.wav", std::process::id()));
    save_wav(&tmp, samples, sample_rate)?;

    // 2. Run fpcalc
    let fpcalc = find_fpcalc().ok_or_else(|| anyhow!("fpcalc not found. Install Chromaprint: https://acoustid.org/chromaprint or https://github.com/acoustid/chromaprint/releases"))?;
    let out = Command::new(&fpcalc).arg("-json").arg(&tmp).output().context("run fpcalc")?;
    if !out.status.success() {
        anyhow::bail!("fpcalc failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    let v: Value = serde_json::from_slice(&out.stdout)?;
    let fingerprint = v.get("fingerprint").and_then(|x| x.as_str()).ok_or_else(|| anyhow!("no fingerprint in fpcalc output"))?.to_string();
    let duration = v.get("duration").and_then(|x| x.as_u64()).unwrap_or((samples.len() / sample_rate as usize) as u64) as u32;

    // Cleanup temp
    let _ = std::fs::remove_file(&tmp);

    // 3. Call AcoustID
    let client = reqwest::Client::new();
    let params = [("client", api_key), ("meta", "recordings+releasegroups+compress"), ("duration", &duration.to_string()), ("fingerprint", &fingerprint)];
    // Use POST to avoid URL length limits
    let resp = client.post("https://api.acoustid.org/v2/lookup")
        .form(&params)
        .send().await?;
    let txt = resp.text().await?;
    let j: Value = serde_json::from_str(&txt).context("parse acoustid json")?;
    if j.get("status").and_then(|s| s.as_str()) != Some("ok") {
        anyhow::bail!("AcoustID error: {}", txt);
    }
    let results = j.get("results").and_then(|r| r.as_array()).ok_or_else(|| anyhow!("no results"))?;
    if results.is_empty() { anyhow::bail!("No match on AcoustID"); }
    // Find first with recordings
    for r in results {
        if let Some(recs) = r.get("recordings").and_then(|x| x.as_array()) {
            if let Some(rec) = recs.first() {
                let title = rec.get("title").and_then(|x| x.as_str()).unwrap_or("Unknown").to_string();
                let artist = rec.get("artists").and_then(|a| a.as_array()).and_then(|a| a.first())
                    .and_then(|x| x.get("name")).and_then(|n| n.as_str()).unwrap_or("Unknown").to_string();
                let album = rec.get("releasegroups").and_then(|a| a.as_array()).and_then(|a| a.first())
                    .and_then(|x| x.get("title")).and_then(|t| t.as_str()).map(|s| s.to_string());
                return Ok(AcoustIdResult { title, artist, album });
            }
        }
    }
    anyhow::bail!("AcoustID: no recordings found: {}", txt)
}

fn save_wav(path: &PathBuf, samples: &[i16], rate: u32) -> Result<()> {
    let spec = hound::WavSpec { channels: 1, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec)?;
    for s in samples { w.write_sample(*s)?; }
    w.finalize()?;
    Ok(())
}

fn find_fpcalc() -> Option<String> {
    for name in ["fpcalc", "fpcalc.exe"] {
        if let Ok(p) = which::which(name) { return Some(p.to_string_lossy().to_string()); }
    }
    // Check alongside binary
    let local = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("fpcalc.exe")));
    if let Some(p) = local { if p.exists() { return Some(p.to_string_lossy().to_string()); } }
    None
}
