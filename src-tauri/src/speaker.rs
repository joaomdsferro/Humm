//! State machine for reading text aloud.
//!
//! Owns a dedicated player thread that holds rodio's `OutputStream` and
//! `Sink`. The audio output stream is `!Send` on most platforms, so we
//! never move it across threads — commands flow in over a channel.

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::selection;
use crate::settings::Settings;
use crate::tts_cloud;
use crate::tts_local;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub enum SpeakerState {
    Idle,
    Synthesizing,
    Speaking,
}

enum PlayerCmd {
    Play(Vec<u8>),
    Stop,
}

pub struct Speaker {
    state: Arc<Mutex<SpeakerState>>,
    player_tx: Sender<PlayerCmd>,
}

impl Speaker {
    pub fn new(app: AppHandle) -> Self {
        let state = Arc::new(Mutex::new(SpeakerState::Idle));
        let (tx, rx) = mpsc::channel::<PlayerCmd>();
        let state_for_thread = state.clone();
        let app_for_thread = app.clone();

        thread::spawn(move || {
            let device_sink = match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[Humm] Audio output unavailable: {}", e);
                    return;
                }
            };
            let sink = rodio::Player::connect_new(device_sink.mixer());

            loop {
                // If currently playing, poll the channel briefly so we can
                // detect playback completion and emit Idle.
                let cmd = if sink.empty() {
                    rx.recv().ok()
                } else {
                    match rx.recv_timeout(Duration::from_millis(150)) {
                        Ok(c) => Some(c),
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            if sink.empty() {
                                set_state(&state_for_thread, &app_for_thread, SpeakerState::Idle);
                            }
                            continue;
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                };

                let Some(cmd) = cmd else { break };
                match cmd {
                    PlayerCmd::Play(bytes) => {
                        sink.clear();
                        match rodio::Decoder::try_from(Cursor::new(bytes)) {
                            Ok(decoder) => {
                                sink.append(decoder);
                                sink.play();
                                set_state(
                                    &state_for_thread,
                                    &app_for_thread,
                                    SpeakerState::Speaking,
                                );
                            }
                            Err(e) => {
                                eprintln!("[Humm] Audio decode failed: {}", e);
                                set_state(
                                    &state_for_thread,
                                    &app_for_thread,
                                    SpeakerState::Idle,
                                );
                            }
                        }
                    }
                    PlayerCmd::Stop => {
                        sink.stop();
                        set_state(&state_for_thread, &app_for_thread, SpeakerState::Idle);
                    }
                }
            }
        });

        Self {
            state,
            player_tx: tx,
        }
    }

    pub fn get_state(&self) -> SpeakerState {
        *self.state.lock().unwrap()
    }

    fn set_state(&self, app: &AppHandle, new_state: SpeakerState) {
        set_state(&self.state, app, new_state);
    }

    pub fn stop(&self, app: &AppHandle) {
        let _ = self.player_tx.send(PlayerCmd::Stop);
        self.set_state(app, SpeakerState::Idle);
    }

    /// Capture selection/clipboard, synthesize with the configured engine,
    /// stream into the audio sink. If already speaking or synthesizing,
    /// stop instead.
    pub async fn toggle_read(
        &self,
        app: &AppHandle,
        settings: &Settings,
        app_dir: &PathBuf,
    ) -> Result<String, String> {
        if self.get_state() != SpeakerState::Idle {
            self.stop(app);
            return Ok("stopped".to_string());
        }

        let text = selection::capture_text()?;
        let text = text.trim().to_string();
        if text.is_empty() {
            return Err("Nothing to read".to_string());
        }

        self.set_state(app, SpeakerState::Synthesizing);

        let engine = settings.tts_engine.clone();
        let rate = settings.tts_rate;
        let edge_voice = settings.edge_voice.clone();
        let piper_voice = settings.piper_voice.clone();
        let app_dir_clone = app_dir.clone();
        let text_clone = text.clone();

        // Both engines are blocking — run on the blocking pool.
        let audio = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
            match engine.as_str() {
                "cloud" => tts_cloud::synthesize(&edge_voice, &text_clone, rate),
                "local" => tts_local::synthesize(&app_dir_clone, &piper_voice, &text_clone, rate),
                other => Err(format!("Unknown TTS engine: {}", other)),
            }
        })
        .await
        .map_err(|e| format!("synthesis task panicked: {}", e))?;

        match audio {
            Ok(bytes) => {
                if bytes.is_empty() {
                    self.set_state(app, SpeakerState::Idle);
                    return Err("Synthesizer returned empty audio".to_string());
                }
                self.player_tx
                    .send(PlayerCmd::Play(bytes))
                    .map_err(|e| e.to_string())?;
                Ok("speaking".to_string())
            }
            Err(e) => {
                self.set_state(app, SpeakerState::Idle);
                Err(e)
            }
        }
    }
}

fn set_state(state: &Arc<Mutex<SpeakerState>>, app: &AppHandle, new_state: SpeakerState) {
    let mut s = state.lock().unwrap();
    if *s == new_state {
        return;
    }
    *s = new_state;
    let _ = app.emit("speaker-state", new_state);
    update_overlay(app, new_state);
}

fn update_overlay(app: &AppHandle, state: SpeakerState) {
    let Some(overlay) = app.get_webview_window("overlay") else { return };

    #[cfg(target_os = "linux")]
    {
        // Mirror the recorder: keep overlay shown during synth/speak, hide
        // when idle so focus returns to the user's window.
        match state {
            SpeakerState::Idle => {
                let _ = overlay.hide();
            }
            _ => {
                let _ = overlay.show();
                let _ = overlay.set_focusable(false);
            }
        }
    }

    let label = match state {
        SpeakerState::Idle => "ready",
        SpeakerState::Synthesizing => "synthesizing",
        SpeakerState::Speaking => "speaking",
    };
    let _ = overlay.eval(&format!("window.__setState('{}');", label));
}
