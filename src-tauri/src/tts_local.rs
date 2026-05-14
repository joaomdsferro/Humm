//! Piper text-to-speech (local).
//!
//! Requires the `piper` binary on PATH. Voice files (`.onnx` + `.onnx.json`)
//! live in `<app_dir>/piper-voices/` and are downloaded on demand from
//! the rhasspy/piper-voices HuggingFace repo.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

use crate::downloader::DownloadProgress;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PiperVoice {
    pub id: String,
    pub label: String,
    pub lang: String,
    pub quality: String,
    /// Path under huggingface.co/rhasspy/piper-voices/resolve/main/
    pub hf_path: String,
}

/// Curated set of voices. Add more as needed.
pub fn voice_catalog() -> Vec<PiperVoice> {
    vec![
        PiperVoice {
            id: "en_US-amy-medium".into(),
            label: "Amy (US English, medium)".into(),
            lang: "en_US".into(),
            quality: "medium".into(),
            hf_path: "en/en_US/amy/medium/en_US-amy-medium".into(),
        },
        PiperVoice {
            id: "en_US-ryan-high".into(),
            label: "Ryan (US English, high)".into(),
            lang: "en_US".into(),
            quality: "high".into(),
            hf_path: "en/en_US/ryan/high/en_US-ryan-high".into(),
        },
        PiperVoice {
            id: "en_GB-alan-medium".into(),
            label: "Alan (British English, medium)".into(),
            lang: "en_GB".into(),
            quality: "medium".into(),
            hf_path: "en/en_GB/alan/medium/en_GB-alan-medium".into(),
        },
        PiperVoice {
            id: "pt_PT-tugão-medium".into(),
            label: "Tugão (European Portuguese, medium)".into(),
            lang: "pt_PT".into(),
            quality: "medium".into(),
            hf_path: "pt/pt_PT/tugão/medium/pt_PT-tugão-medium".into(),
        },
        PiperVoice {
            id: "pt_BR-faber-medium".into(),
            label: "Faber (Brazilian Portuguese, medium)".into(),
            lang: "pt_BR".into(),
            quality: "medium".into(),
            hf_path: "pt/pt_BR/faber/medium/pt_BR-faber-medium".into(),
        },
        PiperVoice {
            id: "es_ES-davefx-medium".into(),
            label: "Davefx (Spanish, medium)".into(),
            lang: "es_ES".into(),
            quality: "medium".into(),
            hf_path: "es/es_ES/davefx/medium/es_ES-davefx-medium".into(),
        },
        PiperVoice {
            id: "fr_FR-siwis-medium".into(),
            label: "Siwis (French, medium)".into(),
            lang: "fr_FR".into(),
            quality: "medium".into(),
            hf_path: "fr/fr_FR/siwis/medium/fr_FR-siwis-medium".into(),
        },
        PiperVoice {
            id: "de_DE-thorsten-medium".into(),
            label: "Thorsten (German, medium)".into(),
            lang: "de_DE".into(),
            quality: "medium".into(),
            hf_path: "de/de_DE/thorsten/medium/de_DE-thorsten-medium".into(),
        },
    ]
}

fn voices_dir(app_dir: &Path) -> PathBuf {
    app_dir.join("piper-voices")
}

pub fn voice_files(app_dir: &Path, voice_id: &str) -> (PathBuf, PathBuf) {
    let dir = voices_dir(app_dir);
    let onnx = dir.join(format!("{}.onnx", voice_id));
    let json = dir.join(format!("{}.onnx.json", voice_id));
    (onnx, json)
}

pub fn voice_downloaded(app_dir: &Path, voice_id: &str) -> bool {
    let (onnx, json) = voice_files(app_dir, voice_id);
    onnx.exists() && json.exists()
}

pub async fn download_voice(
    app: AppHandle,
    app_dir: PathBuf,
    voice_id: String,
) -> Result<(), String> {
    let voice = voice_catalog()
        .into_iter()
        .find(|v| v.id == voice_id)
        .ok_or_else(|| format!("Unknown voice: {}", voice_id))?;

    let dir = voices_dir(&app_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let base = format!(
        "https://huggingface.co/rhasspy/piper-voices/resolve/main/{}",
        voice.hf_path
    );
    let onnx_url = format!("{}.onnx", base);
    let json_url = format!("{}.onnx.json", base);

    let (onnx_path, json_path) = voice_files(&app_dir, &voice.id);

    // Tiny config file: just fetch.
    let client = reqwest::Client::new();
    let json_bytes = client
        .get(&json_url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    std::fs::write(&json_path, &json_bytes).map_err(|e| e.to_string())?;

    // ONNX model: large, stream with progress events.
    crate::downloader::download_model(app.clone(), &onnx_url, &onnx_path).await?;

    let _ = app.emit(
        "tts-voice-download-progress",
        DownloadProgress {
            downloaded: 1,
            total: 1,
            percent: 100.0,
        },
    );
    Ok(())
}

/// Map rate slider (-50..50) to piper --length-scale.
fn rate_to_length_scale(rate: i32) -> f32 {
    let r = rate.clamp(-50, 50) as f32;
    (1.0 - r / 100.0).clamp(0.5, 1.5)
}

/// Synthesize text → WAV bytes via the `piper` binary on PATH.
pub fn synthesize(
    app_dir: &Path,
    voice_id: &str,
    text: &str,
    rate: i32,
) -> Result<Vec<u8>, String> {
    let (onnx, _) = voice_files(app_dir, voice_id);
    if !onnx.exists() {
        return Err(format!(
            "Voice '{}' not downloaded. Download it in settings first.",
            voice_id
        ));
    }

    let length_scale = rate_to_length_scale(rate);
    let sample_rate = read_voice_sample_rate(app_dir, voice_id).unwrap_or(22_050);

    let mut child = Command::new("piper")
        .args([
            "--model",
            onnx.to_str().unwrap(),
            "--output-raw",
            "--length-scale",
            &format!("{:.2}", length_scale),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "Failed to launch `piper`: {}. Install it (e.g. `pip install piper-tts` or download from github.com/rhasspy/piper) and make sure it is on PATH.",
                e
            )
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    }

    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "piper failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // --output-raw writes signed 16-bit little-endian PCM mono.
    // Wrap it in a WAV header so rodio can decode it. Sample rate is set
    // by the voice config; the standard Piper voices we ship are 22050 Hz.
    Ok(wrap_pcm_as_wav(&output.stdout, sample_rate, 1))
}

fn read_voice_sample_rate(app_dir: &Path, voice_id: &str) -> Option<u32> {
    let (_, json_path) = voice_files(app_dir, voice_id);
    let text = std::fs::read_to_string(&json_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    json["audio"]["sample_rate"].as_u64().map(|n| n as u32)
}

fn wrap_pcm_as_wav(pcm: &[u8], sample_rate: u32, channels: u16) -> Vec<u8> {
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;
    let data_len = pcm.len() as u32;
    let chunk_size = 36 + data_len;

    let mut buf = Vec::with_capacity(44 + pcm.len());
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&chunk_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_len.to_le_bytes());
    buf.extend_from_slice(pcm);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_mapping() {
        assert!((rate_to_length_scale(0) - 1.0).abs() < 1e-6);
        assert!(rate_to_length_scale(50) < 1.0);
        assert!(rate_to_length_scale(-50) > 1.0);
    }

    #[test]
    fn wav_header_well_formed() {
        let pcm = vec![0u8; 100];
        let wav = wrap_pcm_as_wav(&pcm, 22050, 1);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
    }
}
