# Humm -- Dictation App Design (Current)

## Overview

Humm is a minimal Tauri 2 desktop dictation app. It captures microphone audio via global hotkey, transcribes using either a local whisper.cpp sidecar or Groq cloud Whisper, performs lightweight cleanup, and auto-pastes into the currently focused app.

Personal-use tool. No accounts, no cloud sync, no history.

## Architecture

### Runtime Model

- Frontend: Vite + vanilla TypeScript settings window
- Backend: Rust orchestration and platform integration
- Overlay: separate always-on-top status pill window
- Transcription backends: local whisper.cpp sidecar or Groq REST API

### Backend Modules

- `main.rs`: Tauri entrypoint, command handlers, global hotkey wiring, overlay creation
- `recorder.rs`: state machine (`Ready -> Recording -> Transcribing -> Ready`) and orchestration
- `audio.rs`: audio capture (`cpal`) + WAV writing (`hound`)
- `transcribe_local.rs`: whisper.cpp sidecar invocation
- `transcribe_groq.rs`: Groq Whisper HTTP transcription
- `paste.rs`: platform-specific paste implementation
- `cleanup.rs`: post-transcription cleanup
- `settings.rs`: persisted config JSON in app config directory
- `downloader.rs`: model downloads with progress events

## Recording Flow

1. User triggers the configured global hotkey.
2. App enters `Recording` and captures mic audio.
3. User stops recording (toggle mode: hotkey press; push-to-talk: release).
4. App saves temporary WAV and enters `Transcribing`.
5. Engine routing:
   - `local`: whisper.cpp sidecar with selected local model
   - `cloud`: Groq Whisper API with user API key
6. Raw text is cleaned.
7. App returns to `Ready` and auto-pastes cleaned text if non-empty.
8. Temporary WAV is removed.

## Transcription Engines

### Local (whisper.cpp sidecar)

- Bundled Linux sidecar binary expected at `src-tauri/binaries/whisper-cpp-x86_64-unknown-linux-gnu` for Linux builds
- Model files stored in app config directory as `ggml-{size}.bin`
- Supported model names in settings: `tiny`, `base`, `small`, `medium`, `large`
- Model download emits frontend progress events

### Cloud (Groq Whisper)

- Uses Groq API (`whisper-large-v3-turbo`)
- Requires user-provided API key
- Uploads recorded WAV as multipart form data

## Text Cleanup

Runs after transcription and before paste:

1. Trim surrounding whitespace
2. Normalize repeated spaces
3. Capitalize sentence starts
4. Ensure terminal punctuation

No LLM post-processing.

## Auto-Paste

Platform-specific paste path:

- macOS: `osascript`
- Windows: `enigo`
- Linux: `wl-copy` + `wtype` (fallback: `ydotool`)

## Frontend UI

Single-page settings window with:

- Recording state indicator (`Ready`, `Recording`, `Transcribing`)
- Microphone selection
- Engine selection (local/cloud)
- Local model selection + download progress
- Groq API key input
- Recording mode (`toggle` / `push-to-talk`)
- Hotkey capture and save

Overlay window (`src/overlay.html`) shows lightweight recording/transcribing state outside the main settings window.

## Settings Storage

JSON file path:

- Linux: `~/.config/com.Humm.app/config.json`
- macOS: `~/Library/Application Support/com.Humm.app/config.json`
- Windows: `%APPDATA%/com.Humm.app/config.json`

Example:

```json
{
  "microphone": "default",
  "engine": "local",
  "whisperModel": "small",
  "groqApiKey": "",
  "recordingMode": "toggle",
  "hotkey": "CmdOrCtrl+Shift+Space"
}
```

## Platform Support

- Linux (primary development target)
- macOS
- Windows

## Out of Scope

- Transcription history and sync
- Accounts/authentication
- LLM rewriting of output text
- Real-time/streaming transcription
