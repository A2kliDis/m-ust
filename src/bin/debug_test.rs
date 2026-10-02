use hound::WavReader;

fn main() -> anyhow::Result<()> {
    let p = "debug_last.wav";
    let mut reader = WavReader::open(p)?;
    let spec = reader.spec();
    let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
    let rms = (samples.iter().map(|s| (*s as f32/32768.0).powi(2)).sum::<f32>() / samples.len() as f32).sqrt();
    let max = samples.iter().map(|s| s.abs() as i32).max().unwrap_or(0);
    println!("spec={:?} n={} rms={:.4} max={} dur={:.2}", spec, samples.len(), rms, max, samples.len() as f32 / spec.sample_rate as f32);
    
    // Import our fingerprint
    let sig = m_ust::fingerprint::generate_shazam_signature(&samples);
    let total_peaks: usize = sig.frequency_band_to_sound_peaks.values().map(|v| v.len()).sum();
    println!("bands={} total_peaks={}", sig.frequency_band_to_sound_peaks.len(), total_peaks);
    for (b, v) in &sig.frequency_band_to_sound_peaks {
        println!(" band {}: {} peaks", b, v.len());
    }
    let uri = sig.encode_to_uri()?;
    println!("uri len={} prefix={}", uri.len(), &uri[..60]);
    
    // Try Shazam
    let rt = tokio::runtime::Runtime::new()?;
    let res = rt.block_on(m_ust::api::recognize_with_shazam(&sig));
    match res {
        Ok(r) => println!("SUCCESS: {} - {} album {:?} url {:?}", r.artist, r.title, r.album, r.url),
        Err(e) => println!("FAILED: {}", e),
    }
    Ok(())
}
