# Building and Installing Humm (Linux / CachyOS)

## First-time build and install

### 1. Build

```bash
npm run tauri build
```

Takes ~5–10 minutes on first run. Output lands in `src-tauri/target/release/bundle/`.

### 2. Install the AppImage

```bash
chmod +x src-tauri/target/release/bundle/appimage/Humm_*.AppImage
mkdir -p ~/.local/bin
cp src-tauri/target/release/bundle/appimage/Humm_*.AppImage ~/.local/bin/humm.AppImage
```

### 3. Register in the app launcher (one-time)

```bash
cat > ~/.local/share/applications/humm.desktop << 'EOF'
[Desktop Entry]
Name=Humm
Exec=/home/joaomdsferro/.local/bin/humm.AppImage
Icon=humm
Type=Application
Categories=Utility;AudioVideo;
EOF
```

Humm will now appear in the CachyOS app launcher.

---

## Rebuilding after code changes

```bash
npm run tauri build && cp src-tauri/target/release/bundle/appimage/Humm_*.AppImage ~/.local/bin/humm.AppImage
```

Subsequent builds are faster (incremental Rust compilation).

---

## Notes

- The whisper-cpp binary must be present at `src-tauri/binaries/whisper-cpp-x86_64-unknown-linux-gnu` before building — it gets bundled into the AppImage.
- The `.desktop` file only needs to be created once. The `Exec` path points to a fixed location (`~/.local/bin/humm.AppImage`), so rebuilds that copy to the same path are picked up automatically without touching the `.desktop` file.
- Avoid `debtap` (`.deb` → Arch package conversion) for a dev workflow — it adds unnecessary steps on every rebuild.
