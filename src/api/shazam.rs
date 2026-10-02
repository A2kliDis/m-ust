use anyhow::{Result, anyhow};
use serde_json::Value;
use crate::fingerprint::shazam::ShazamSignature;

#[derive(Clone, Debug)]
pub struct ShazamResult {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub url: Option<String>,
    pub cover_url: Option<String>,
    pub raw: Value,
}

/// Send fingerprint to Shazam (100% free, no key)
/// Matches SongRec's communication.rs exactly
pub async fn recognize_with_shazam(sig: &ShazamSignature) -> Result<ShazamResult> {
    let uri = sig.encode_to_uri().map_err(|e| anyhow!("encode uri: {}", e))?;
    let samplems = (sig.number_samples as f32 / sig.sample_rate_hz as f32 * 1000.0) as u32;

    let timestamp_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u32;

    let body = serde_json::json!({
        "geolocation": { "altitude": 0, "latitude": 0, "longitude": 0 },
        "signature": {
            "uri": uri,
            "samplems": samplems,
            "timestamp": timestamp_ms
        },
        "timestamp": timestamp_ms,
        "timezone": "Europe/Paris"
    });

    let uuid1 = uuid::Uuid::new_v4().hyphenated().to_string().to_uppercase();
    let uuid2 = uuid::Uuid::new_v4().hyphenated().to_string();
    let url = format!("https://amp.shazam.com/discovery/v5/en/US/android/-/tag/{}/{}", uuid1, uuid2);

    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(20)).build()?;

    // Random user agent like SongRec
    let agents = [
        "Shazam/13.25.0 Android/8.1",
        "SongRec 0.7.4",
        "Mozilla/5.0 (Linux; Android 8.0) Shazam",
    ];
    let ua = agents[rand::random::<usize>() % agents.len()];

    let resp = client.post(&url)
        .header("Content-Type", "application/json")
        .header("User-Agent", ua)
        .header("Content-Language", "en_US")
        .query(&[("sync","true"),("webv3","true"),("sampling","true"),("connected",""),("shazamapiversion","v3"),("sharehub","true"),("video","v3")])
        .json(&body)
        .send().await.map_err(|e| anyhow!("network: {}", e))?;

    if !resp.status().is_success() {
        let txt = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Shazam HTTP {}: {}", 0, txt));
    }
    let v: Value = resp.json().await?;
    // SongRec checks track directly, then matches[0].track
    if let Some(track) = v.get("track").cloned() {
        return parse_track(&track, v);
    }
    if let Some(m) = v.get("matches").and_then(|m| m.as_array()).and_then(|a| a.first()).cloned() {
        if let Some(track) = m.get("track").cloned() { return parse_track(&track, v); }
        // Some responses have no track but retryms hint -> not found
        if m.get("retryms").is_some() {
            return Err(anyhow!("Song not recognized (retryms): {}", v));
        }
    }
    Err(anyhow!("Song not recognized: {}", v))
}

fn parse_track(track: &Value, raw: Value) -> Result<ShazamResult> {
    let title = track.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
    let artist = track.get("subtitle").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
    let album = track.get("sections").and_then(|s| s.as_array())
        .and_then(|a| a.iter().find(|x| x.get("type").and_then(|t| t.as_str()) == Some("SONG")))
        .and_then(|s| s.get("metadata")).and_then(|m| m.as_array())
        .and_then(|a| a.iter().find(|m| m.get("title").and_then(|t| t.as_str()) == Some("Album")))
        .and_then(|m| m.get("text").and_then(|t| t.as_str())).map(|s| s.to_string());
    let url = track.get("url").and_then(|v| v.as_str()).map(|s| s.to_string())
        .or_else(|| track.get("share").and_then(|s| s.get("href")).and_then(|h| h.as_str()).map(|s| s.to_string()));
    // Cover art: prefer high-quality, fall back to standard then share image
    let cover_url = track.get("images").and_then(|i| i.get("coverarthq")).and_then(|v| v.as_str())
        .or_else(|| track.get("images").and_then(|i| i.get("coverart")).and_then(|v| v.as_str()))
        .or_else(|| track.get("share").and_then(|s| s.get("image")).and_then(|h| h.as_str()))
        .map(|s| s.to_string());
    Ok(ShazamResult { title, artist, album, url, cover_url, raw })
}
