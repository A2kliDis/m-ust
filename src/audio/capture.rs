//! Audio capture: Microphone + System Loopback + Per-App (Windows)
use anyhow::{Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
pub enum CaptureMode {
    Microphone(String),          // input device name
    SystemLoopback(String),      // device name or "Default" - captures what you hear in headphones/speakers
    AppLoopback(String),         // specific app (process name / pid)
}

#[derive(Clone, Debug)]
pub struct CapturedAudio {
    pub samples: Vec<i16>, // mono 16kHz
    #[allow(dead_code)]
    pub sample_rate: u32,
    pub duration_secs: f32,
}

pub struct AudioCapture {
    pub mode: CaptureMode,
    pub duration: Duration,
}

impl AudioCapture {
    pub fn new(mode: CaptureMode, duration_secs: u64) -> Self {
        Self { mode, duration: Duration::from_secs(duration_secs) }
    }

    pub async fn record(&self) -> Result<CapturedAudio> {
        match &self.mode {
            CaptureMode::Microphone(dev) => self.record_cpal_input(dev).await,
            CaptureMode::SystemLoopback(dev) => {
                #[cfg(target_os = "windows")]
                { self.record_wasapi_loopback(dev.clone(), None).await }
                #[cfg(not(target_os = "windows"))]
                { self.record_cpal_monitor().await }
            },
            CaptureMode::AppLoopback(app) => {
                #[cfg(target_os = "windows")]
                { self.record_wasapi_loopback("Default".into(), Some(app.clone())).await }
                #[cfg(not(target_os = "windows"))]
                { Err(anyhow!("Per-app capture is only supported on Windows. Use SystemLoopback on Linux/macOS")) }
            }
        }
    }

    // --- CPAL Microphone ---
    async fn record_cpal_input(&self, device_name: &str) -> Result<CapturedAudio> {
        let host = cpal::default_host();
        let device = if device_name.is_empty() {
            host.default_input_device().ok_or_else(|| anyhow!("No default input device"))?
        } else {
            host.input_devices()?.find(|d| d.name().map(|n| n==device_name).unwrap_or(false))
                .ok_or_else(|| anyhow!("Device not found: {}", device_name))?
        };

        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
        let buffer_clone = buffer.clone();

        let err_fn = |e| eprintln!("Stream error: {}", e);

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| buffer_clone.lock().unwrap().extend_from_slice(data),
                err_fn, None
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    let mut b = buffer_clone.lock().unwrap();
                    b.extend(data.iter().map(|s| *s as f32 / 32768.0));
                },
                err_fn, None
            )?,
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _| {
                    let mut b = buffer_clone.lock().unwrap();
                    b.extend(data.iter().map(|s| (*s as f32 - 32768.0)/32768.0));
                },
                err_fn, None
            )?,
            _ => return Err(anyhow!("Unsupported sample format")),
        };

        stream.play()?;
        tokio::time::sleep(self.duration).await;
        drop(stream);

        let raw = buffer.lock().unwrap().clone();
        let mut mono = downmix_and_resample(&raw, channels, sample_rate, 16000);
        let peak = mono.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        if peak > 0.001 && peak < 0.5 {
            let gain = (0.9 / peak).min(8.0);
            for v in &mut mono { *v *= gain; }
        }
        let samples_i16: Vec<i16> = mono.into_iter().map(|f| (f.clamp(-1.0,1.0)*32767.0) as i16).collect();
        let _ = save_debug_wav(&samples_i16, 16000);
        Ok(CapturedAudio { samples: samples_i16.clone(), sample_rate: 16000, duration_secs: samples_i16.len() as f32 / 16000.0 })
    }

    // --- CPAL Monitor (Linux PipeWire/Pulse) ---
    #[cfg(not(target_os = "windows"))]
    async fn record_cpal_monitor(&self) -> Result<CapturedAudio> {
        let host = cpal::default_host();
        let dev = host.input_devices()?.find(|d| d.name().map(|n| n.to_lowercase().contains("monitor")).unwrap_or(false))
            .or_else(|| host.default_input_device());
        let name = dev.as_ref().and_then(|d| d.name().ok()).unwrap_or_default();
        if name.to_lowercase().contains("monitor") {
            self.record_cpal_input(&name).await
        } else {
            self.record_cpal_input("").await
        }
    }

    // --- WASAPI Loopback (Windows) — captures exactly what you hear in headphones/speakers ---
    #[cfg(target_os = "windows")]
    async fn record_wasapi_loopback(&self, device_name: String, app: Option<String>) -> Result<CapturedAudio> {
        let duration = self.duration;
        // Try per-app process loopback first if app is Some (include child processes - browsers use child pids)
        if let Some(app_str) = app.clone() {
            if let Some(pid) = Self::parse_pid(&app_str) {
                let res = tokio::task::spawn_blocking(move || wasapi_process_loopback(pid, duration))
                    .await
                    .map_err(|e| anyhow!("join: {}", e))?;
                match res {
                    Ok(samples) => return Ok(CapturedAudio { samples: samples.clone(), sample_rate: 16000, duration_secs: samples.len() as f32 / 16000.0 }),
                    Err(_) => {
                        // Per-app failed — fallback to System Loopback
                    }
                }
            }
        }
        let dev_clone = device_name.clone();
        let res = tokio::task::spawn_blocking(move || wasapi_loopback_blocking_for_device(dev_clone, duration))
            .await
            .map_err(|e| anyhow!("join: {}", e))?;
        match res {
            Ok(samples) => Ok(CapturedAudio { samples: samples.clone(), sample_rate: 16000, duration_secs: samples.len() as f32 / 16000.0 }),
            Err(e) => {
                let host = cpal::default_host();
                if let Ok(devs) = host.input_devices() {
                    for dev in devs {
                        if let Ok(name) = dev.name() {
                            let low = name.to_lowercase();
                            if low.contains("stereo mix") {
                                return self.record_cpal_input(&name).await;
                            }
                        }
                    }
                }
                Err(anyhow!("Loopback failed on '{}' ({}). Try another output device.", device_name, e))
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn parse_pid(s: &str) -> Option<u32> {
        // format "chrome.exe (PID 1234)" or "1234"
        if let Some(start) = s.find("(PID ") {
            let rest = &s[start+5..];
            if let Some(end) = rest.find(')') {
                return rest[..end].trim().parse().ok();
            }
        }
        s.trim().parse().ok()
    }
}

fn downmix_and_resample(input: &[f32], channels: usize, from_rate: u32, to_rate: u32) -> Vec<f32> {
    if input.is_empty() { return vec![]; }
    let ch = channels.max(1);
    let frames = input.len() / ch;
    let mut mono = Vec::with_capacity(frames);
    for i in 0..frames {
        let mut sum = 0.0;
        for c in 0..ch { sum += input[i*ch + c]; }
        mono.push(sum / ch as f32);
    }
    if from_rate == to_rate { return mono; }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (mono.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f64 * ratio;
        let idx = pos as usize;
        let frac = (pos - idx as f64) as f32;
        let a = mono[idx.min(mono.len()-1)];
        let b = mono[(idx+1).min(mono.len()-1)];
        out.push(a*(1.0-frac) + b*frac);
    }
    out
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
fn wasapi_loopback_blocking(duration: Duration) -> Result<Vec<i16>> {
    wasapi_loopback_blocking_for_device("Default".into(), duration)
}

#[cfg(target_os = "windows")]
fn wasapi_loopback_blocking_for_device(device_name: String, duration: Duration) -> Result<Vec<i16>> {
    use wasapi::{Direction, ShareMode, get_default_device, initialize_mta, SampleType, DeviceCollection};

    let _ = initialize_mta();

    let device = if device_name == "Default" || device_name.is_empty() {
        get_default_device(&Direction::Render).map_err(|e| anyhow!("get default render device: {}", e))?
    } else {
        let coll = DeviceCollection::new(&Direction::Render).map_err(|e| anyhow!("enum devices: {}", e))?;
        coll.get_device_with_name(&device_name).map_err(|e| anyhow!("device '{}' not found: {}", device_name, e))?
    };
    let mut client = device.get_iaudioclient().map_err(|e| anyhow!("get iaudioclient: {}", e))?;
    let mix_format = client.get_mixformat().map_err(|e| anyhow!("get mixformat: {}", e))?;

    let samplerate = mix_format.get_samplespersec();
    let channels = mix_format.get_nchannels() as usize;
    let blockalign = mix_format.get_blockalign() as usize;
    let bitspersample = mix_format.get_bitspersample();
    let sample_type = mix_format.get_subformat().unwrap_or(SampleType::Float);

    // For loopback, we reuse the mixformat and request Capture direction
    // This automatically sets AUDCLNT_STREAMFLAGS_LOOPBACK | EVENTCALLBACK
    let (_def, min) = client.get_periods().map_err(|e| anyhow!("get periods: {}", e))?;
    client.initialize_client(&mix_format, min, &Direction::Capture, &ShareMode::Shared, true)
        .map_err(|e| anyhow!("initialize loopback client: {} (try playing audio first)", e))?;

    let h_event = client.set_get_eventhandle().map_err(|e| anyhow!("event: {}", e))?;
    let capture = client.get_audiocaptureclient().map_err(|e| anyhow!("capture client: {}", e))?;

    client.start_stream().map_err(|e| anyhow!("start: {}", e))?;

    let start = Instant::now();
    let mut all_f32: Vec<f32> = Vec::with_capacity((samplerate as usize * duration.as_secs() as usize * channels) );

    while start.elapsed() < duration {
        // Wait for data (100ms timeout keeps UI responsive even with silence)
        let _ = h_event.wait_for_event(200);

        // Drain all available packets
        loop {
            let frames_opt = capture.get_next_nbr_frames().map_err(|e| anyhow!("get next frames: {}", e))?;
            let frames = frames_opt.unwrap_or(0);
            if frames == 0 { break; }

            let bytes_needed = frames as usize * blockalign;
            let mut buf = vec![0u8; bytes_needed];
            let (read_frames, flags) = capture.read_from_device(&mut buf).map_err(|e| anyhow!("read: {}", e))?;

            if read_frames == 0 { break; }

            if flags.silent {
                // Device is silent (no audio playing) -> push zeros to keep timing
                all_f32.extend(vec![0.0; read_frames as usize * channels]);
            } else {
                // Convert raw bytes to f32
                if sample_type == SampleType::Float && bitspersample == 32 {
                    for chunk in buf.chunks_exact(4) {
                        let v = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                        all_f32.push(v);
                    }
                } else if sample_type == SampleType::Int && bitspersample == 16 {
                    for chunk in buf.chunks_exact(2) {
                        let v = i16::from_le_bytes([chunk[0], chunk[1]]);
                        all_f32.push(v as f32 / 32768.0);
                    }
                } else if sample_type == SampleType::Int && bitspersample == 24 {
                    for chunk in buf.chunks_exact(3) {
                        let v = (chunk[0] as i32) | ((chunk[1] as i32) << 8) | ((chunk[2] as i32) << 16);
                        let v = if v & 0x800000 != 0 { v | !0xFFFFFF } else { v };
                        all_f32.push(v as f32 / 8388608.0);
                    }
                } else if sample_type == SampleType::Int && bitspersample == 32 {
                    for chunk in buf.chunks_exact(4) {
                        let v = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                        all_f32.push(v as f32 / 2147483648.0);
                    }
                } else {
                    anyhow::bail!("unsupported loopback format {:?} {}bit", sample_type, bitspersample);
                }
            }
            // Only 16-bit mono is needed for Shazam, but we keep raw interleaved for now
            // Trim if we exceeded duration (avoid huge buffer)
            if all_f32.len() > samplerate as usize * duration.as_secs() as usize * channels * 2 { break; }
        }
    }

    client.stop_stream().ok();

    if all_f32.is_empty() {
        anyhow::bail!("captured silence (no audio played during recording). Play a song in your headphones then press [r]");
    }

    // Downmix to mono + resample to 16k for Shazam
    let mut mono = downmix_and_resample(&all_f32, channels, samplerate, 16000);
    // Auto-gain: normalize quiet loopback captures (YouTube often at -30dB)
    let peak = mono.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
    if peak > 0.001 && peak < 0.5 {
        let gain = (0.9 / peak).min(8.0); // max 18dB boost to avoid noise explosion
        for v in &mut mono { *v *= gain; }
    }
    let out: Vec<i16> = mono.into_iter().map(|f| (f.clamp(-1.0,1.0)*32767.0) as i16).collect();
    let _ = save_debug_wav(&out, 16000);
    Ok(out)
}

#[cfg(target_os = "windows")]
fn wasapi_process_loopback(pid: u32, duration: Duration) -> Result<Vec<i16>> {
    use wasapi::{Direction, ShareMode, SampleType, WaveFormat, initialize_mta};
    use wasapi::AudioClient;
    let _ = initialize_mta();
    // Try process loopback (Windows 10 2004+)
    let mut client = AudioClient::new_application_loopback_client(pid, true)
        .map_err(|e| anyhow!("process loopback not supported (need Win10 2004+): {}", e))?;
    // Desired format - use 48k float stereo as safe choice (process loopback doesn't support get_mixformat)
    let format = WaveFormat::new(32, 32, &SampleType::Float, 48000, 2, None);
    // For process loopback, hns = 200000 (20ms), autoconvert true
    // Use the 3-arg initialize if available, otherwise try 5-arg; try both via match
    // Try 5-arg first (wasapi 0.15)
    let init = client.initialize_client(&format, 200000, &Direction::Capture, &ShareMode::Shared, true);
    if init.is_err() {
        // Fallback: try calling with minimal period (some wrappers use different sig)
        anyhow::bail!("process loopback init failed: {:?}", init.err());
    }
    let h_event = client.set_get_eventhandle().map_err(|e| anyhow!("event: {}", e))?;
    let capture = client.get_audiocaptureclient().map_err(|e| anyhow!("capture: {}", e))?;
    client.start_stream().map_err(|e| anyhow!("start: {}", e))?;
    let start = Instant::now();
    let mut all_f32: Vec<f32> = Vec::new();
    while start.elapsed() < duration {
        let _ = h_event.wait_for_event(200);
        loop {
            let frames = capture.get_next_nbr_frames().map_err(|e| anyhow!("next frames: {}", e))?.unwrap_or(0);
            if frames == 0 { break; }
            let mut buf = vec![0u8; frames as usize * 8]; // 2ch *4 bytes
            let (read, flags) = capture.read_from_device(&mut buf).map_err(|e| anyhow!("read: {}", e))?;
            if read == 0 { break; }
            if flags.silent {
                all_f32.extend(vec![0.0; read as usize * 2]);
            } else {
                for chunk in buf[..read as usize*8].chunks_exact(4) {
                    all_f32.push(f32::from_le_bytes([chunk[0],chunk[1],chunk[2],chunk[3]]));
                }
            }
        }
    }
    client.stop_stream().ok();
    if all_f32.is_empty() { anyhow::bail!("no audio from PID {} — is it playing?", pid); }
    let mut mono = downmix_and_resample(&all_f32, 2, 48000, 16000);
    let peak = mono.iter().map(|v| v.abs()).fold(0.0, f32::max);
    if peak > 0.001 && peak < 0.5 { let g=(0.9/peak).min(8.0); for v in &mut mono {*v*=g;} }
    let out: Vec<i16> = mono.into_iter().map(|f| (f.clamp(-1.0,1.0)*32767.0) as i16).collect();
    let _ = save_debug_wav(&out, 16000);
    Ok(out)
}

fn save_debug_wav(samples: &[i16], rate: u32) -> Result<()> {
    let spec = hound::WavSpec { channels: 1, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).join("debug_last.wav");
    let mut w = hound::WavWriter::create(&path, spec)?;
    for s in samples { w.write_sample(*s)?; }
    w.finalize()?;
    Ok(())
}
