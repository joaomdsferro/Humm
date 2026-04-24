# Humm

Forked from [Humm](https://github.com/albertshiney/typr), Humm is the repo where I will continue to work on the project with my own vision :)

## Usage

Prerequisites:

- Node.js (v18+ recommended) and `npm` or `pnpm`
- Rust toolchain (for the Tauri backend): `rustup`, `cargo`
- libssl, libgtk (typical Linux desktop deps) — install via your distro package manager

Run (development):

1. Install JavaScript dependencies:

	`npm install`

2. Run the frontend dev server and Tauri backend (two terminals):

	Terminal 1 (frontend):

	`npm run dev`

	Terminal 2 (Tauri backend + app):

	`cargo build`  # first-time build for native binaries
	`npm run tauri dev`

Build (production):

1. Build the web assets and native app:

	`npm run build`
	`npm run tauri build`

Notes:

- This project was developed on Linux (CachyOS/Arch). It should work on other Linux distributions but may require installing additional system libraries.
- If you need local speech binaries, see `src-tauri/binaries/` for included helper binaries.
