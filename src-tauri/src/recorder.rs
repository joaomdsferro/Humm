use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

use crate::audio::AudioRecorder;
use crate::cleanup::cleanup_text;
use crate::paste::paste_text;
use crate::settings::Settings;
use crate::transcribe_local;
use crate::transcribe_groq;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum RecordingState {
    Ready,
    Recording,
    Transcribing,
}

pub const OVERLAY_WINDOW_TITLE: &str = "Humm Status Overlay";

fn update_overlay(app: &AppHandle, state: &RecordingState) {
    let Some(overlay) = app.get_webview_window("overlay") else {
        println!("[Humm] update_overlay: overlay window not found");
        return;
    };
    let s = match state {
        RecordingState::Ready => "ready",
        RecordingState::Recording => "recording",
        RecordingState::Transcribing => "transcribing",
    };
    let visible = overlay.is_visible().unwrap_or(false);
    println!("[Humm] update_overlay: state={} visible={}", s, visible);
    if let Ok(pos) = overlay.outer_position() {
        println!("[Humm] update_overlay: pos=({},{})", pos.x, pos.y);
    }
    if let Ok(sz) = overlay.outer_size() {
        println!("[Humm] update_overlay: size={}x{}", sz.width, sz.height);
    }
    match overlay.eval(&format!("window.__setState('{}');", s)) {
        Ok(_) => println!("[Humm] update_overlay: eval ok"),
        Err(e) => println!("[Humm] update_overlay: eval FAILED: {}", e),
    }
}


pub struct Recorder {
    state: Arc<Mutex<RecordingState>>,
    audio_recorder: Arc<Mutex<AudioRecorder>>,
}

impl Recorder {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(RecordingState::Ready)),
            audio_recorder: Arc::new(Mutex::new(AudioRecorder::new())),
        }
    }

    pub fn get_state(&self) -> RecordingState {
        self.state.lock().unwrap().clone()
    }

    pub fn start_recording(&self, app: &AppHandle, mic_name: &str) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if *state != RecordingState::Ready {
            return Err("Already recording or transcribing".to_string());
        }

        let mut recorder = self.audio_recorder.lock().unwrap();
        recorder.start(mic_name)?;

        *state = RecordingState::Recording;
        let _ = app.emit("recording-state", RecordingState::Recording);

        // On Linux the overlay is hidden after each paste to return focus to the
        // user's window.  Re-show it now (before updating CSS) so the pill is
        // visible during recording/transcription.
        #[cfg(target_os = "linux")]
        if let Some(ov) = app.get_webview_window("overlay") {
            let _ = ov.show();
            let _ = ov.set_focusable(false);

            // Non-Hyprland Wayland / X11: try to reposition at top-right of the
            // monitor the compositor just placed the overlay on.
            if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_err() {
                let monitor = ov
                    .current_monitor()
                    .ok()
                    .flatten()
                    .or_else(|| app.primary_monitor().ok().flatten());
                if let Some(mon) = monitor {
                    let mon_pos = mon.position();
                    let mon_sz  = mon.size();
                    let scale   = mon.scale_factor();
                    let pill_px = (170.0 * scale).round() as i32;
                    let margin  = (50.0  * scale).round() as i32;
                    let tx = mon_pos.x + mon_sz.width as i32 - pill_px - margin;
                    let ty = mon_pos.y + margin;
                    let _ = ov.set_position(PhysicalPosition::new(tx, ty));
                }
            }
        }

        update_overlay(app, &RecordingState::Recording);
        Ok(())
    }

    pub async fn stop_and_transcribe(
        &self,
        app: &AppHandle,
        settings: &Settings,
        app_dir: &PathBuf,
    ) -> Result<String, String> {
        // Stop recording
        {
            let mut state = self.state.lock().unwrap();
            if *state != RecordingState::Recording {
                return Err("Not currently recording".to_string());
            }
            *state = RecordingState::Transcribing;
            let _ = app.emit("recording-state", RecordingState::Transcribing);
            update_overlay(app, &RecordingState::Transcribing);
        }

        let temp_path = app_dir.join("temp_recording.wav");

        // Save audio
        {
            let mut recorder = self.audio_recorder.lock().unwrap();
            recorder.stop_and_save(&temp_path)?;
        }

        // Transcribe
        let raw_text = match settings.engine.as_str() {
            "local" => {
                let model_path = app_dir.join(transcribe_local::model_filename(&settings.whisper_model));
                transcribe_local::transcribe_local(app, &model_path, &temp_path).await?
            }
            "cloud" => {
                transcribe_groq::transcribe_groq(&settings.groq_api_key, &temp_path).await?
            }
            _ => return Err(format!("Unknown engine: {}", settings.engine)),
        };

        // Cleanup temp file
        let _ = std::fs::remove_file(&temp_path);

        // Clean up text
        let cleaned = cleanup_text(&raw_text);

        // Update visual state to Ready before paste so the pill disappears
        // while focus is being restored.
        {
            let mut state = self.state.lock().unwrap();
            *state = RecordingState::Ready;
            let _ = app.emit("recording-state", RecordingState::Ready);
            update_overlay(app, &RecordingState::Ready);
        }

        // Auto-paste
        if !cleaned.is_empty() {
            // On Linux/Wayland the overlay window can hold keyboard focus even
            // with set_focusable(false) on some compositors.  Hiding it forces
            // the compositor to return focus to the user's previous window
            // before we inject the Ctrl+V keystroke.
            #[cfg(target_os = "linux")]
            {
                if let Some(ov) = app.get_webview_window("overlay") {
                    let _ = ov.hide();
                }
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
            #[cfg(not(target_os = "linux"))]
            std::thread::sleep(std::time::Duration::from_millis(200));

            paste_text(&cleaned)?;
        }

        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_ready() {
        let recorder = Recorder::new();
        assert_eq!(recorder.get_state(), RecordingState::Ready);
    }
}
