//! Microsoft Edge "Read Aloud" TTS — free, no API key.
//!
//! Runs the synchronous `msedge-tts` client on a blocking thread because
//! the crate uses its own (non-tokio) runtime internally.

use msedge_tts::tts::{client::connect, SpeechConfig};

/// Curated subset of Edge voices. The full list is fetched live via the
/// `list_edge_voices` Tauri command when the UI needs it.
#[derive(Clone, serde::Serialize)]
pub struct EdgeVoiceLite {
    pub short_name: String,
    pub locale: String,
    pub gender: String,
    pub friendly_name: String,
}

pub fn synthesize(voice: &str, text: &str, rate: i32) -> Result<Vec<u8>, String> {
    let voice = voice.to_string();
    let text = text.to_string();
    let config = SpeechConfig {
        voice_name: voice,
        // 24 kHz MP3 — good quality, modest size, decoded by rodio.
        audio_format: "audio-24khz-48kbitrate-mono-mp3".to_string(),
        pitch: 0,
        rate: rate.clamp(-50, 50),
        volume: 0,
    };

    let mut client = connect().map_err(|e| format!("Edge TTS connect failed: {}", e))?;
    let audio = client
        .synthesize(&text, &config)
        .map_err(|e| format!("Edge TTS synthesize failed: {}", e))?;
    Ok(audio.audio_bytes)
}

pub fn list_voices() -> Result<Vec<EdgeVoiceLite>, String> {
    let voices =
        msedge_tts::voice::get_voices_list().map_err(|e| format!("Edge voice list: {}", e))?;
    Ok(voices
        .into_iter()
        .filter_map(|v| {
            let short = v.short_name.clone()?;
            Some(EdgeVoiceLite {
                short_name: short,
                locale: v.locale.unwrap_or_default(),
                gender: v.gender.unwrap_or_default(),
                friendly_name: v.friendly_name.unwrap_or_default(),
            })
        })
        .collect())
}
