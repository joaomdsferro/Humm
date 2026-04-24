# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

Use `pnpm` (not `npm`) — project uses `pnpm@10.33.2` as package manager.

```bash
# Dev (two terminals)
pnpm dev                   # frontend Vite dev server
pnpm tauri dev             # Tauri backend + app window

# Production build
pnpm run build             # tsc + vite build
pnpm tauri build           # full native binary → src-tauri/target/release/Humm

# Rust tests only
cd src-tauri && cargo test

# Run a single Rust test
cd src-tauri && cargo test test_name
```

## Architecture

Humm is a Tauri 2 desktop dictation app. Press a global hotkey → record mic audio → transcribe → auto-paste into the focused window.

### Two processes

**Rust backend** (`src-tauri/src/`):
- `main.rs` — Tauri app entry, `AppState` (Recorder + Settings), all `#[tauri::command]` handlers, global hotkey registration, overlay window creation
- `recorder.rs` — `Recorder` struct drives the state machine: `Ready → Recording → Transcribing → Ready`. Calls `start_recording` / `stop_and_transcribe` which orchestrates audio, transcription, cleanup, and paste.
- `audio.rs` — `AudioRecorder` wraps `cpal` for mic capture, saves WAV via `hound`
- `transcribe_local.rs` — runs bundled `whisper-cpp` sidecar binary via `tauri-plugin-shell`
- `transcribe_groq.rs` — sends WAV to Groq cloud API (`whisper-large-v3-turbo`)
- `paste.rs` — platform-specific paste: macOS uses `osascript`, Windows uses `enigo`, Linux uses `wl-copy` + `wtype` (fallback: `ydotool`)
- `cleanup.rs` — normalizes whitespace, capitalizes sentences, ensures trailing punctuation
- `settings.rs` — `Settings` struct, persisted as JSON at `~/.config/com.Humm.app/config.json`
- `downloader.rs` — streams Whisper model files from HuggingFace, emits `download-progress` events

**TypeScript frontend** (`src/main.ts`, `index.html`):
- Settings UI — mic selector, engine toggle (local/cloud), model selector + download, Groq API key, recording mode (toggle/push-to-talk), hotkey capture
- Listens to `recording-state` events from Rust to update status indicator

**Overlay window** (`src/overlay.html`):
- Separate always-on-top, non-focusable, transparent pill that shows recording/transcribing/ready state
- On Linux, hidden between sessions so Wayland assigns position on `show()`; Hyprland gets `windowrulev2` rules injected via `hyprctl`

### Key data flows

1. Hotkey press → `handle_shortcut` in `main.rs` → `do_toggle_recording` → `Recorder::start_recording` or `stop_and_transcribe`
2. `stop_and_transcribe`: stops audio → saves `temp_recording.wav` → picks engine from settings → gets raw text → `cleanup_text` → hides overlay (Linux) → `paste_text`
3. Frontend can also invoke `toggle_recording` command directly

### Settings

`engine`: `"local"` | `"cloud"`  
`whisper_model`: `"tiny"` | `"base"` | `"small"` | `"medium"` | `"large"` (model files: `ggml-{size}.bin` in app config dir)  
`recordingMode`: `"toggle"` | `"push-to-talk"`  
`hotkey`: Tauri accelerator string e.g. `"CmdOrCtrl+Shift+Space"`

### Linux specifics

- Paste requires `wl-copy` and `wtype` (preferred) or `ydotoold` daemon
- Overlay uses hide/show cycle per recording session for Wayland focus management
- Hyprland: `windowrulev2` rules auto-injected on startup if `HYPRLAND_INSTANCE_SIGNATURE` is set
- whisper-cpp sidecar binary: `src-tauri/binaries/whisper-cpp-x86_64-unknown-linux-gnu` must exist before building
