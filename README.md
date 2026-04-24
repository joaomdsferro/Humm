# Humm

Forked from [Typr](https://github.com/albertshiney/typr), Humm is a Tauri 2 desktop dictation app.

Press a global hotkey, record mic audio, transcribe (local Whisper sidecar or Groq cloud), and auto-paste into the focused window.

## Package Manager

Use `pnpm` (not `npm`).

## Prerequisites

- Node.js (v18+ recommended)
- `pnpm` (project uses `pnpm@10.33.2`)
- Rust toolchain (`rustup`, `cargo`)
- Linux desktop dependencies (such as OpenSSL/GTK), installed via your distro package manager

## Development

Install dependencies:

```bash
pnpm install
```

Run in development with two terminals:

Terminal 1 (frontend):

```bash
pnpm dev
```

Terminal 2 (Tauri backend + app window):

```bash
pnpm tauri dev
```

## Build

Build web assets:

```bash
pnpm run build
```

Build native binary:

```bash
pnpm tauri build
```

Binary output: `src-tauri/target/release/Humm`

## Notes

- Linux is a supported platform in this fork.
- Linux paste support requires `wl-copy` and `wtype` (preferred), or `ydotool`/`ydotoold`.
- The whisper sidecar binary must exist at `src-tauri/binaries/whisper-cpp-x86_64-unknown-linux-gnu` before building on Linux.
