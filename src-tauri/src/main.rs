#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use humm_lib::audio;
use humm_lib::downloader;
use humm_lib::recorder::{Recorder, RecordingState, OVERLAY_WINDOW_TITLE};
use humm_lib::settings::Settings;
use humm_lib::transcribe_local;

struct AppState {
    recorder: Recorder,
    settings: Mutex<Settings>,
    app_dir: PathBuf,
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
                .stop_and_transcribe(app, &settings, &state.app_dir)
                .await?;
            Ok(result)
        }
        RecordingState::Transcribing => {
            Err("Currently transcribing, please wait".to_string())
        }
    }
}

fn main() {
    let app_dir = get_app_dir();
    let settings = Settings::load(&app_dir);
    let initial_hotkey = settings.hotkey.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            recorder: Recorder::new(),
            settings: Mutex::new(settings),
            app_dir,
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            list_microphones,
            get_recording_state,
            check_model_downloaded,
            download_model,
            toggle_recording,
        ])
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

            let handle = app.handle().clone();

            println!("[Humm] Registering global shortcut: {}", initial_hotkey);

            match app.global_shortcut().on_shortcut(
                initial_hotkey.as_str(),
                move |_app, shortcut, event| {
                    println!("[Humm] Hotkey event: {:?} state={:?}", shortcut, event.state);
                    let handle = handle.clone();
                    let state = handle.state::<AppState>();
                    let mode = state.settings.lock().unwrap().recording_mode.clone();
                    println!("[Humm] Recording mode: {}", mode);

                    match event.state {
                        ShortcutState::Pressed => {
                            tauri::async_runtime::spawn(async move {
                                let state = handle.state::<AppState>();
                                match mode.as_str() {
                                    "toggle" => {
                                        println!("[Humm] Toggle mode: calling do_toggle_recording");
                                        match do_toggle_recording(&handle, state.inner()).await {
                                            Ok(result) => println!("[Humm] Toggle result: {}", result),
                                            Err(e) => eprintln!("[Humm] Toggle error: {}", e),
                                        }
                                    }
                                    "push-to-talk" => {
                                        let current = state.recorder.get_state();
                                        println!("[Humm] PTT mode, current state: {:?}", current);
                                        if current == RecordingState::Ready {
                                            let mic = state
                                                .settings
                                                .lock()
                                                .unwrap()
                                                .microphone
                                                .clone();
                                            match state.recorder.start_recording(&handle, &mic) {
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
                                    let state = handle.state::<AppState>();
                                    let current = state.recorder.get_state();
                                    if current == RecordingState::Recording {
                                        let settings =
                                            state.settings.lock().unwrap().clone();
                                        match state.recorder.stop_and_transcribe(
                                            &handle,
                                            &settings,
                                            &state.app_dir,
                                        ).await {
                                            Ok(result) => println!("[Humm] Transcription: {}", result),
                                            Err(e) => eprintln!("[Humm] Transcription error: {}", e),
                                        }
                                    }
                                });
                            }
                        }
                    }
                },
            ) {
                Ok(_) => println!("[Humm] Global shortcut registered successfully"),
                Err(e) => eprintln!("[Humm] ERROR: Failed to register global shortcut: {}", e),
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
