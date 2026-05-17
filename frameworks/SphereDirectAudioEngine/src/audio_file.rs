//! Audio file decoder for the native playback engine.
//!
//! **WAV/WAVE** — decoded by an inline RIFF/WAVE parser (fast, zero extra deps).
//! **Everything else** — decoded via `symphonia` (MP3, FLAC, OGG Vorbis, AIFF).
//!
//! The result is always interleaved `f32` samples normalised to `−1.0 … 1.0`.
//! Decoding happens on the control thread; the audio callback only reads the
//! finished `AudioFileBuffer` through an `Arc` — no allocation at runtime.

use std::fs::File;
use std::io;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

// ── Public API ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AudioFileBuffer {
    pub sample_rate: u32,
    pub channels:    usize,
    pub frames:      usize,
    /// Interleaved PCM samples, normalised to `−1.0 … 1.0`.
    pub samples:     Vec<f32>,
}

/// Load an audio file from `path` into a decoded `AudioFileBuffer`.
///
/// Supported extensions: `wav`, `wave`, `mp3`, `flac`, `ogg`, `oga`,
/// `aiff`, `aif`.
///
/// Returns an error string on failure; the caller logs it and skips the clip.
pub fn load_audio_file(path: &str) -> Result<AudioFileBuffer, String> {
    let p = Path::new(path);
    let ext = p
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        // Fast path — hand-written RIFF/WAVE parser (no symphonia overhead).
        "wav" | "wave" => load_wav(p),

        // Symphonia handles everything else.
        "mp3" | "flac" | "ogg" | "oga" | "aiff" | "aif" => load_via_symphonia(p),

        other => Err(format!("unsupported native audio format '{other}'")),
    }
}

// ── Symphonia decoder ──────────────────────────────────────────────────────────

fn load_via_symphonia(path: &Path) -> Result<AudioFileBuffer, String> {
    let src = File::open(path).map_err(|e| format!("Cannot open '{}': {e}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions { enable_gapless: true, ..Default::default() },
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("Format probe failed: {e}"))?;

    let mut format = probed.format;

    // Pick the first decodable audio track.
    let track = format
        .tracks()
        .iter()
        .find(|t| {
            t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL
        })
        .ok_or_else(|| "No decodable audio track found".to_string())?
        .clone();

    let track_id  = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| "Track has no sample rate".to_string())?;
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(2);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Failed to create codec decoder: {e}"))?;

    let mut all_samples: Vec<f32> = Vec::new();
    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            // Clean EOF.
            Err(SymphoniaError::IoError(ref e)) if e.kind() == io::ErrorKind::UnexpectedEof => {
                break;
            }
            // The codec / format needs a reset (e.g. after a seek or stream error).
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(e) => return Err(format!("Packet read error: {e}")),
        };

        // Skip packets that belong to other tracks (e.g. album art).
        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(audio_buf_ref) => {
                // Initialise the sample buffer on first decoded block.
                if sample_buf.is_none() {
                    let spec = *audio_buf_ref.spec();
                    sample_buf = Some(SampleBuffer::<f32>::new(
                        audio_buf_ref.capacity() as u64,
                        spec,
                    ));
                }
                if let Some(buf) = &mut sample_buf {
                    buf.copy_interleaved_ref(audio_buf_ref);
                    all_samples.extend_from_slice(buf.samples());
                }
            }
            // Benign decode errors — skip the packet and keep going.
            Err(SymphoniaError::IoError(_)) | Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(format!("Decode error: {e}")),
        }
    }

    let frames = if channels > 0 { all_samples.len() / channels } else { 0 };
    Ok(AudioFileBuffer {
        sample_rate,
        channels,
        frames,
        samples: all_samples,
    })
}

// ── Hand-written RIFF/WAVE parser ─────────────────────────────────────────────
//
// Supports PCM 8 / 16 / 24 / 32-bit integer and IEEE float 32-bit.
// Used instead of symphonia for WAV to avoid the extra dependency overhead on
// the most common format.

fn load_wav(path: &Path) -> Result<AudioFileBuffer, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read failed: {e}"))?;
    if bytes.len() < 44 {
        return Err("file too small for WAV".to_string());
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".to_string());
    }

    let mut cursor = 12usize;
    let mut fmt: Option<WavFmt> = None;
    let mut data_range: Option<(usize, usize)> = None;

    while cursor + 8 <= bytes.len() {
        let id  = &bytes[cursor..cursor + 4];
        let len = read_u32_le(&bytes, cursor + 4)? as usize;
        let body = cursor + 8;
        let end  = body.saturating_add(len);
        if end > bytes.len() {
            return Err("truncated WAV chunk".to_string());
        }

        match id {
            b"fmt " => {
                if len < 16 {
                    return Err("invalid fmt chunk".to_string());
                }
                fmt = Some(WavFmt {
                    audio_format:   read_u16_le(&bytes, body)?,
                    channels:       read_u16_le(&bytes, body + 2)? as usize,
                    sample_rate:    read_u32_le(&bytes, body + 4)?,
                    bits_per_sample: read_u16_le(&bytes, body + 14)?,
                });
            }
            b"data" => {
                data_range = Some((body, len));
            }
            _ => {}
        }

        cursor = end + (len & 1); // skip padding byte for odd-length chunks
    }

    let fmt = fmt.ok_or_else(|| "missing fmt chunk".to_string())?;
    let (data_start, data_len) =
        data_range.ok_or_else(|| "missing data chunk".to_string())?;
    if fmt.channels == 0 || fmt.sample_rate == 0 {
        return Err("invalid channel count or sample rate".to_string());
    }

    let bytes_per_sample = match fmt.bits_per_sample {
        8  => 1usize,
        16 => 2,
        24 => 3,
        32 => 4,
        bits => return Err(format!("unsupported WAV bit depth: {bits}")),
    };
    let bytes_per_frame = fmt.channels * bytes_per_sample;
    if bytes_per_frame == 0 || data_len < bytes_per_frame {
        return Err("empty WAV data".to_string());
    }

    let frames       = data_len / bytes_per_frame;
    let sample_count = frames * fmt.channels;
    let mut samples  = Vec::with_capacity(sample_count);

    let mut offset = data_start;
    for _ in 0..sample_count {
        let value = match (fmt.audio_format, fmt.bits_per_sample) {
            // PCM integer
            (1, 8)  => (bytes[offset] as f32 - 128.0) / 128.0,
            (1, 16) => read_i16_le(&bytes, offset)? as f32 / 32_768.0,
            (1, 24) => read_i24_le(&bytes, offset)? as f32 / 8_388_608.0,
            (1, 32) => read_i32_le(&bytes, offset)? as f32 / 2_147_483_648.0,
            // IEEE float
            (3, 32) => f32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]),
            (format, _) => return Err(format!("unsupported WAV format code: {format}")),
        };
        samples.push(value.clamp(-1.0, 1.0));
        offset += bytes_per_sample;
    }

    Ok(AudioFileBuffer {
        sample_rate: fmt.sample_rate,
        channels:    fmt.channels,
        frames,
        samples,
    })
}

// ── Byte-level helpers ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct WavFmt {
    audio_format:    u16,
    channels:        usize,
    sample_rate:     u32,
    bits_per_sample: u16,
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let b = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| "unexpected EOF reading u16".to_string())?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let b = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "unexpected EOF reading u32".to_string())?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn read_i16_le(bytes: &[u8], offset: usize) -> Result<i16, String> {
    let b = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| "unexpected EOF reading i16".to_string())?;
    Ok(i16::from_le_bytes([b[0], b[1]]))
}

fn read_i24_le(bytes: &[u8], offset: usize) -> Result<i32, String> {
    let b = bytes
        .get(offset..offset + 3)
        .ok_or_else(|| "unexpected EOF reading i24".to_string())?;
    let raw = ((b[2] as i32) << 16) | ((b[1] as i32) << 8) | b[0] as i32;
    Ok((raw << 8) >> 8)
}

fn read_i32_le(bytes: &[u8], offset: usize) -> Result<i32, String> {
    let b = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "unexpected EOF reading i32".to_string())?;
    Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
