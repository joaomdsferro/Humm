#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

use humm_lib::audio;
use humm_lib::downloader;
use humm_lib::recorder::{Recorder, RecordingState, OVERLAY_WINDOW_TITLE};
use humm_lib::settings::Settings;
use humm_lib::speaker::{Speaker, SpeakerState};
use humm_lib::transcribe_local;
use humm_lib::tts_cloud;
use humm_lib::tts_local;

struct AppState {
    recorder: Recorder,
    speaker: std::sync::OnceLock<Speaker>,
    settings: Mutex<Settings>,
    app_dir: PathBuf,
    http_client: reqwest::Client,
}

impl AppState {
    fn speaker(&self) -> &Speaker {
        self.speaker.get().expect("Speaker not initialized yet")
    }
}

fn get_app_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.Humm.app")
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: Settings) -> Result<(), String> {
    settings.save(&state.app_dir)?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

#[tauri::command]
fn list_microphones() -> Vec<audio::MicDevice> {
    audio::list_microphones()
}

#[tauri::command]
fn get_recording_state(state: State<AppState>) -> RecordingState {
    state.recorder.get_state()
}

#[tauri::command]
fn check_model_downloaded(state: State<AppState>, model_size: String) -> bool {
    let model_file = transcribe_local::model_filename(&model_size);
    state.app_dir.join(&model_file).exists()
}

#[tauri::command]
async fn download_model(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    model_size: String,
) -> Result<(), String> {
    let url = transcribe_local::model_download_url(&model_size);
    let model_file = transcribe_local::model_filename(&model_size);
    let dest = state.app_dir.join(&model_file);
    downloader::download_model(app, &url, &dest).await
}

#[tauri::command]
async fn toggle_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    do_toggle_recording(&app, &state).await
}

#[tauri::command]
async fn toggle_read(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    do_toggle_read(&app, &state).await
}

#[tauri::command]
fn stop_reading(app: tauri::AppHandle, state: State<AppState>) {
    state.speaker().stop(&app);
}

#[tauri::command]
fn get_speaker_state(state: State<AppState>) -> SpeakerState {
    state.speaker().get_state()
}

#[tauri::command]
fn list_piper_voices() -> Vec<tts_local::PiperVoice> {
    tts_local::voice_catalog()
}

#[tauri::command]
fn check_piper_voice_downloaded(state: State<AppState>, voice_id: String) -> bool {
    tts_local::voice_downloaded(&state.app_dir, &voice_id)
}

#[tauri::command]
async fn download_piper_voice(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    voice_id: String,
) -> Result<(), String> {
    let app_dir = state.app_dir.clone();
    tts_local::download_voice(app, app_dir, voice_id).await
}

#[tauri::command]
async fn list_edge_voices() -> Result<Vec<tts_cloud::EdgeVoiceLite>, String> {
    tokio::task::spawn_blocking(tts_cloud::list_voices)
        .await
        .map_err(|e| e.to_string())?
}

async fn do_toggle_read(
    app: &tauri::AppHandle,
    state: &AppState,
) -> Result<String, String> {
    let settings = state.settings.lock().unwrap().clone();
    state
        .speaker()
        .toggle_read(app, &settings, &state.app_dir)
        .await
}

/// Shared logic for toggle recording, used by both the Tauri command and hotkey handler.
async fn do_toggle_recording(
    app: &tauri::AppHandle,
    state: &AppState,
) -> Result<String, String> {
    let current_state = state.recorder.get_state();
    match current_state {
        RecordingState::Ready => {
            let mic = state.settings.lock().unwrap().microphone.clone();
            state.recorder.start_recording(app, &mic)?;
            Ok("recording".to_string())
        }
        RecordingState::Recording => {
            let settings = state.settings.lock().unwrap().clone();
            let result = state
                .recorder
                .stop_and_transcribe(app, &settings, &state.app_dir, &state.http_client)
                .await?;
            Ok(result)
        }
        RecordingState::Transcribing => {
            Err("Currently transcribing, please wait".to_string())
        }
    }
}

fn handle_shortcut(app: &tauri::AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    let app = app.clone();
    let state = app.state::<AppState>();
    let (record_hk, read_hk, mode) = {
        let s = state.settings.lock().unwrap();
        (s.hotkey.clone(), s.read_hotkey.clone(), s.recording_mode.clone())
    };
    println!("[Humm] Hotkey event: {:?} state={:?}", shortcut, event.state);

    // Route to TTS toggle if this is the read hotkey (press only).
    if shortcut_matches(shortcut, &read_hk) {
        if event.state == ShortcutState::Pressed {
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                match do_toggle_read(&app, state.inner()).await {
                    Ok(r) => println!("[Humm] Read toggle: {}", r),
                    Err(e) => eprintln!("[Humm] Read toggle error: {}", e),
                }
            });
        }
        return;
    }

    if !shortcut_matches(shortcut, &record_hk) {
        return;
    }

    match event.state {
        ShortcutState::Pressed => {
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                match mode.as_str() {
                    "toggle" => {
                        println!("[Humm] Toggle mode: calling do_toggle_recording");
                        match do_toggle_recording(&app, state.inner()).await {
                            Ok(result) => println!("[Humm] Toggle result: {}", result),
                            Err(e) => eprintln!("[Humm] Toggle error: {}", e),
                        }
                    }
                    "push-to-talk" => {
                        let current = state.recorder.get_state();
                        println!("[Humm] PTT mode, current state: {:?}", current);
                        if current == RecordingState::Ready {
                            let mic = state.settings.lock().unwrap().microphone.clone();
                            match state.recorder.start_recording(&app, &mic) {
                                Ok(_) => println!("[Humm] Recording started"),
                                Err(e) => eprintln!("[Humm] Start recording error: {}", e),
                            }
                        }
                    }
                    _ => {}
                }
            });
        }
        ShortcutState::Released => {
            if mode == "push-to-talk" {
                tauri::async_runtime::spawn(async move {
                    let state = app.state::<AppState>();
                    // Normal case: already Recording, proceed immediately.
                    // Race case: key released during stream init — wait once for it to finish.
                    if state.recorder.get_state() != RecordingState::Recording {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        if state.recorder.get_state() != RecordingState::Recording {
                            return;
                        }
                    }
                    let settings = state.settings.lock().unwrap().clone();
                    match state
                        .recorder
                        .stop_and_transcribe(&app, &settings, &state.app_dir, &state.http_client)
                        .await
                    {
                        Ok(result) => println!("[Humm] Transcription: {}", result),
                        Err(e) => eprintln!("[Humm] Transcription error: {}", e),
                    }
                });
            }
        }
    }
}

fn register_hotkey(app: &tauri::AppHandle, hotkey: &str) -> Result<(), String> {
    println!("[Humm] Registering global shortcut: {}", hotkey);
    app.global_shortcut()
        .on_shortcut(hotkey, handle_shortcut)
        .map_err(|e| format!("Failed to register hotkey '{}': {}", hotkey, e))
}

fn shortcut_matches(actual: &Shortcut, accelerator: &str) -> bool {
    match accelerator.parse::<Shortcut>() {
        Ok(parsed) => &parsed == actual,
        Err(_) => false,
    }
}

#[tauri::command]
fn update_hotkey(
    app: tauri::AppHandle,
    state: State<AppState>,
    new_hotkey: String,
) -> Result<(), String> {
    let old_hotkey = state.settings.lock().unwrap().hotkey.clone();

    if let Err(e) = app.global_shortcut().unregister(old_hotkey.as_str()) {
        eprintln!("[Humm] Warning: failed to unregister '{}': {}", old_hotkey, e);
    }

    if let Err(e) = register_hotkey(&app, &new_hotkey) {
        let _ = register_hotkey(&app, &old_hotkey);
        return Err(e);
    }

    let mut settings = state.settings.lock().unwrap();
    settings.hotkey = new_hotkey;
    settings.save(&state.app_dir)?;
    println!("[Humm] Hotkey updated successfully");
    Ok(())
}

#[tauri::command]
fn update_read_hotkey(
    app: tauri::AppHandle,
    state: State<AppState>,
    new_hotkey: String,
) -> Result<(), String> {
    let old_hotkey = state.settings.lock().unwrap().read_hotkey.clone();

    if let Err(e) = app.global_shortcut().unregister(old_hotkey.as_str()) {
        eprintln!("[Humm] Warning: failed to unregister read hotkey '{}': {}", old_hotkey, e);
    }

    if let Err(e) = register_hotkey(&app, &new_hotkey) {
        let _ = register_hotkey(&app, &old_hotkey);
        return Err(e);
    }

    let mut settings = state.settings.lock().unwrap();
    settings.read_hotkey = new_hotkey;
    settings.save(&state.app_dir)?;
    println!("[Humm] Read hotkey updated successfully");
    Ok(())
}

fn main() {
    let app_dir = get_app_dir();
    let settings = Settings::load(&app_dir);
    let initial_hotkey = settings.hotkey.clone();
    let initial_read_hotkey = settings.read_hotkey.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            recorder: Recorder::new(),
            speaker: std::sync::OnceLock::new(),
            settings: Mutex::new(settings),
            app_dir,
            http_client: reqwest::Client::new(),
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            list_microphones,
            get_recording_state,
            check_model_downloaded,
            download_model,
            toggle_recording,
            update_hotkey,
            toggle_read,
            stop_reading,
            get_speaker_state,
            list_piper_voices,
            check_piper_voice_downloaded,
            download_piper_voice,
            list_edge_voices,
            update_read_hotkey,
        ])
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::Destroyed = event {
                    window.app_handle().exit(0);
                }
            }
        })
        .setup(move |app| {
            // Overlay window stays mapped for the entire app lifetime so it
            // never steals focus. Visual state is controlled via CSS opacity.
            let monitor = app.primary_monitor().ok().flatten();
            let (x, y) = if let Some(m) = &monitor {
                let size = m.size();
                let scale = m.scale_factor();
                let logical_w = size.width as f64 / scale;
                (logical_w - 220.0, 50.0)
            } else {
                (1380.0, 50.0)
            };
            println!("[Humm] Overlay target position: ({}, {})", x, y);

            let overlay = WebviewWindowBuilder::new(
                app,
                "overlay",
                WebviewUrl::App("src/overlay.html".into()),
            )
            .title(OVERLAY_WINDOW_TITLE)
            .inner_size(170.0, 44.0)
            .position(x, y)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .shadow(false)
            .build();

            match overlay {
                Ok(win) => {
                    println!("[Humm] Overlay window created");
                    let _ = win.set_focusable(false);
                    #[cfg(not(target_os = "linux"))]
                    let _ = win.set_ignore_cursor_events(true);

                    // On Linux, start hidden so that show() in start_recording triggers
                    // a real Wayland map event and the compositor assigns a position.
                    // On other platforms keep always-visible (opacity controls visibility).
                    #[cfg(target_os = "linux")]
                    {
                        let hide_res = win.hide();
                        println!("[Humm] Overlay hidden at startup (Linux): {:?}", hide_res);

                        // Hyprland: inject windowrulev2 rules so the overlay floats,
                        // pins to all workspaces (always-on-top), never steals focus,
                        // and appears at the top-right corner of whichever monitor it maps on.
                        // Coordinates are monitor-relative logical pixels.
                        if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
                            let t = OVERLAY_WINDOW_TITLE;
                            let rules = [
                                format!("float, title:{t}"),
                                format!("pin, title:{t}"),
                                format!("noinitialfocus, title:{t}"),
                                format!("move {} 50, title:{t}", x as i32),
                            ];
                            for rule in &rules {
                                let ok = std::process::Command::new("hyprctl")
                                    .args(["keyword", "windowrulev2", rule])
                                    .output()
                                    .map(|o| o.status.success())
                                    .unwrap_or(false);
                                println!("[Humm] Hyprland rule '{rule}': ok={ok}");
                            }
                        }
                    }

                    if let Ok(pos) = win.outer_position() {
                        println!("[Humm] Overlay actual position: ({}, {})", pos.x, pos.y);
                    }
                    if let Ok(sz) = win.outer_size() {
                        println!("[Humm] Overlay actual size: {}x{}", sz.width, sz.height);
                    }
                }
                Err(e) => eprintln!("[Humm] Failed to create overlay: {}", e),
            }

            // Speaker needs an AppHandle to emit state events, so create
            // it now that the app is set up.
            let state: tauri::State<AppState> = app.state();
            let _ = state.speaker.set(Speaker::new(app.handle().clone()));

            match register_hotkey(app.handle(), &initial_hotkey) {
                Ok(_) => println!("[Humm] Global shortcut registered successfully"),
                Err(e) => eprintln!("[Humm] ERROR: {}", e),
            }

            match register_hotkey(app.handle(), &initial_read_hotkey) {
                Ok(_) => println!("[Humm] Read hotkey registered: {}", initial_read_hotkey),
                Err(e) => eprintln!("[Humm] ERROR registering read hotkey: {}", e),
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
