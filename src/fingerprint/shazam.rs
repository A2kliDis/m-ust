//! Shazam fingerprint - 100% Rust, SongRec compatible
//! Based on Wang 2003 + https://github.com/marin-m/SongRec/src/fingerprinting/

use std::collections::HashMap;
use std::io::{Cursor, Seek, SeekFrom, Write};
use byteorder::{LittleEndian, WriteBytesExt};
use crc32fast::Hasher;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chfft::RFft1D;
use rustfft::num_complex::Complex;

const DATA_URI_PREFIX: &str = "data:audio/vnd.shazam.sig;base64,";

#[derive(Clone, Debug)]
pub struct FrequencyPeak {
    pub fft_pass_number: u32,
    pub peak_magnitude: u16,
    pub corrected_peak_frequency_bin: u16,
}

pub type FrequencyBand = u32;

#[derive(Clone, Debug)]
pub struct DecodedSignature {
    pub sample_rate_hz: u32,
    pub number_samples: u32,
    pub frequency_band_to_sound_peaks: HashMap<FrequencyBand, Vec<FrequencyPeak>>,
}

// Keep old name for compatibility with app.rs
pub type ShazamSignature = DecodedSignature;

fn hanning_window() -> Vec<f32> {
    (0..2048).map(|n| 0.5 * (1.0 - (2.0*std::f32::consts::PI * n as f32 / 2047.0).cos())).collect()
}

impl DecodedSignature {
    pub fn encode_to_binary(&self) -> anyhow::Result<Vec<u8>> {
        let mut cursor = Cursor::new(vec![]);
        cursor.write_u32::<LittleEndian>(0xcafe2580)?; // magic1
        cursor.write_u32::<LittleEndian>(0)?; // crc32 placeholder
        cursor.write_u32::<LittleEndian>(0)?; // size_minus_header placeholder
        cursor.write_u32::<LittleEndian>(0x94119c00)?; // magic2
        cursor.write_u32::<LittleEndian>(0)?; // void1
        cursor.write_u32::<LittleEndian>(0)?;
        cursor.write_u32::<LittleEndian>(0)?;
        let shifted = match self.sample_rate_hz {
            8000 => 1, 11025 => 2, 16000 => 3, 32000 => 4, 44100 => 5, 48000 => 6,
            _ => anyhow::bail!("Invalid sample rate {}", self.sample_rate_hz),
        } << 27;
        cursor.write_u32::<LittleEndian>(shifted)?;
        cursor.write_u32::<LittleEndian>(0)?; // void2
        cursor.write_u32::<LittleEndian>(0)?;
        cursor.write_u32::<LittleEndian>(self.number_samples + (self.sample_rate_hz as f32 * 0.24) as u32)?;
        cursor.write_u32::<LittleEndian>((15 << 19) + 0x40000)?;

        cursor.write_u32::<LittleEndian>(0x40000000)?;
        cursor.write_u32::<LittleEndian>(0)?; // size_minus_header again

        let mut sorted: Vec<_> = self.frequency_band_to_sound_peaks.iter().collect();
        sorted.sort_by(|a,b| a.0.cmp(b.0));

        for (band, peaks) in sorted {
            let mut pk_cursor = Cursor::new(vec![]);
            let mut fft_pass = 0u32;
            for p in peaks {
                assert!(p.fft_pass_number >= fft_pass);
                if p.fft_pass_number - fft_pass >= 255 {
                    pk_cursor.write_u8(0xff)?;
                    pk_cursor.write_u32::<LittleEndian>(p.fft_pass_number)?;
                    fft_pass = p.fft_pass_number;
                }
                pk_cursor.write_u8((p.fft_pass_number - fft_pass) as u8)?;
                pk_cursor.write_u16::<LittleEndian>(p.peak_magnitude)?;
                pk_cursor.write_u16::<LittleEndian>(p.corrected_peak_frequency_bin)?;
                fft_pass = p.fft_pass_number;
            }
            let buf = pk_cursor.into_inner();
            cursor.write_u32::<LittleEndian>(0x60030040 + *band)?;
            cursor.write_u32::<LittleEndian>(buf.len() as u32)?;
            cursor.write_all(&buf)?;
            for _ in 0..((4 - buf.len() as u32 % 4) % 4) { cursor.write_u8(0)?; }
        }

        let size = cursor.position() as u32;
        cursor.seek(SeekFrom::Start(8))?;
        cursor.write_u32::<LittleEndian>(size - 48)?;
        cursor.seek(SeekFrom::Start(48+4))?;
        cursor.write_u32::<LittleEndian>(size - 48)?;
        cursor.seek(SeekFrom::Start(4))?;
        let mut hasher = Hasher::new();
        hasher.update(&cursor.get_ref()[8..]);
        cursor.write_u32::<LittleEndian>(hasher.finalize())?;

        Ok(cursor.into_inner())
    }

    pub fn encode_to_uri(&self) -> anyhow::Result<String> {
        Ok(format!("{}{}", DATA_URI_PREFIX, BASE64.encode(self.encode_to_binary()?)))
    }
}

pub fn generate_shazam_signature(samples_16k_mono: &[i16]) -> ShazamSignature {
    // Trim to middle 12 seconds like SongRec does for files, for live we keep all
    let slice: &[i16] = if samples_16k_mono.len() > 12 * 16000 {
        let mid = samples_16k_mono.len()/2;
        &samples_16k_mono[mid - 6*16000 .. mid + 6*16000]
    } else { samples_16k_mono };

    // Delegate to make_signature_from_buffer
    make_signature_from_buffer(slice)
}

fn make_signature_from_buffer(buf: &[i16]) -> DecodedSignature {
    let hanning = hanning_window();
    let mut fft = RFft1D::<f32>::new(2048);

    let mut ring: Vec<i16> = vec![0; 2048];
    let mut ring_idx = 0usize;
    let mut fft_outputs: Vec<Vec<f32>> = vec![vec![0.0; 1025]; 256];
    let mut fft_idx = 0usize;
    let mut spread: Vec<Vec<f32>> = vec![vec![0.0; 1025]; 256];
    let mut spread_idx = 0usize;
    let mut num_spread = 0u32;
    let mut peaks: HashMap<u32, Vec<FrequencyPeak>> = HashMap::new();

    let mut reordered: Vec<f32> = vec![0.0; 2048];

    for chunk in buf.chunks_exact(128) {
        ring[ring_idx..ring_idx+128].copy_from_slice(chunk);
        ring_idx = (ring_idx + 128) & 2047;

        for i in 0..2048 {
            reordered[i] = ring[(i + ring_idx) & 2047] as f32 * hanning[i];
        }
        let complex_out = fft.forward(&reordered);
        for i in 0..=1024 {
            let mag = (complex_out[i].re*complex_out[i].re + complex_out[i].im*complex_out[i].im) / (1<<17) as f32;
            fft_outputs[fft_idx][i] = mag.max(1e-10);
        }
        fft_idx = (fft_idx + 1) & 255;

        let prev = (fft_idx as i32 -1 & 255) as usize;
        spread[spread_idx].copy_from_slice(&fft_outputs[prev]);
        for p in 0..=1022 { spread[spread_idx][p] = spread[spread_idx][p].max(spread[spread_idx][p+1]).max(spread[spread_idx][p+2]); }
        let copy = spread[spread_idx].clone();
        for &off in &[1,3,6] {
            let idx = (spread_idx as i32 - off & 255) as usize;
            for p in 0..=1024 { if copy[p] > spread[idx][p] { spread[idx][p] = copy[p]; } }
        }
        spread_idx = (spread_idx + 1) & 255;
        num_spread += 1;

        if num_spread >= 46 {
            let fft_46 = (fft_idx as i32 -46 & 255) as usize;
            let spread_49 = (spread_idx as i32 -49 & 255) as usize;
            for bin in 10..=1014 {
                if fft_outputs[fft_46][bin] < 1.0/64.0 { continue; }
                if fft_outputs[fft_46][bin] < spread[spread_49][bin-1] { continue; }
                let mut max_n: f32 = 0.0;
                for &o in &[-10,-7,-4,-3,1,2,5,8] { max_n = max_n.max(spread[spread_49][(bin as i32 + o) as usize]); }
                if fft_outputs[fft_46][bin] <= max_n { continue; }
                let mut max_t: f32 = max_n;
                for &o in &[-53,-45,165,172,179,186,193,200,214,221,228,235,242,249] {
                    let idx = (spread_idx as i32 + o & 255) as usize;
                    max_t = max_t.max(spread[idx][bin-1]);
                }
                if fft_outputs[fft_46][bin] <= max_t { continue; }

                let pass = num_spread - 46;
                let mag = (fft_outputs[fft_46][bin].ln().max(1.0/64.0) * 1477.3 + 6144.0) as i32;
                let mag_before = (fft_outputs[fft_46][bin-1].ln().max(1.0/64.0) * 1477.3 + 6144.0) as i32;
                let mag_after  = (fft_outputs[fft_46][bin+1].ln().max(1.0/64.0) * 1477.3 + 6144.0) as i32;
                let var1 = mag*2 - mag_before - mag_after;
                if var1 == 0 { continue; }
                let var2 = (mag_after - mag_before) * 32 / var1.max(1);
                let freq_bin64 = bin as i32 * 64 + var2; // in 1/64 bin units
                // Convert to Hz: Hz = bin*7.8125 + var2*0.12207
                let hz = freq_bin64 as f32 * 7.8125 / 64.0;
                let band = if hz < 250.0 { continue; } else if hz < 520.0 { 0 } else if hz < 1450.0 { 1 } else if hz < 3500.0 { 2 } else if hz < 5500.0 { 3 } else { continue; };
                let freq = freq_bin64 as u32; // keep corrected bin for storage

                peaks.entry(band).or_default().push(FrequencyPeak {
                    fft_pass_number: pass,
                    peak_magnitude: mag as u16,
                    corrected_peak_frequency_bin: freq as u16,
                });
            }
        }
    }

    let mut sig = DecodedSignature { sample_rate_hz: 16000, number_samples: buf.len() as u32, frequency_band_to_sound_peaks: peaks };
    // For API we need data_uri, but we keep it lazy via encode_to_uri
    // Add helper field via extension trait: we will compute uri on demand in api layer.
    // To keep ShazamSignature compatible, store uri inside frequency_band_to_sound_peaks? No, compute later.
    // Instead we attach uri via a separate method. For now return sig; api will call encode_to_uri.
    // To keep old field `peaks.len()` working, we map.
    sig
}

// Helper to get peaks count and uri for old code
pub trait ShazamExt {
    fn peaks_len(&self) -> usize;
    fn data_uri(&self) -> String;
}
impl ShazamExt for DecodedSignature {
    fn peaks_len(&self) -> usize { self.frequency_band_to_sound_peaks.len() }
    fn data_uri(&self) -> String { self.encode_to_uri().unwrap_or_default() }
}

// Provide compatibility: allow .peaks and .data_uri field access via Deref hack?
// Instead, update api to use encode_to_uri directly. For backward compat, expose helpers.

// Legacy alias for older api code: generate returns DecodedSignature which has .frequency_band_to_sound_peaks
// api will call sig.encode_to_uri().

#[allow(dead_code)]
pub fn build_shazam_payload(_peaks: &HashMap<u32, Vec<(u32,u32)>>, _samples: u32) -> Vec<u8> { vec![] }
