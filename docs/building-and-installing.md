# Building and Installing Humm (Linux / CachyOS)

## First-time build and install

### 1. Build

```bash
npm run tauri build
```

Takes ~5–10 minutes on first run. The binary is output to `src-tauri/target/release/Humm`.

### 2. Symlink the binary (one-time)

```bash
mkdir -p ~/.local/bin
ln -sf /home/joaomdsferro/Documents/Github/Humm/src-tauri/target/release/Humm ~/.local/bin/humm
```

### 3. Register in the app launcher (one-time)

```bash
cat > ~/.local/share/applications/humm.desktop << 'EOF'
[Desktop Entry]
Name=Humm
Exec=/home/joaomdsferro/.local/bin/humm
Icon=humm
Type=Application
Categories=Utility;AudioVideo;
EOF
```

Humm will now appear in the CachyOS app launcher.

---

## Rebuilding after code changes

```bash
npm run tauri build
```

That's it. The symlink always points to the latest binary — no copy step needed.

Subsequent builds are faster (incremental Rust compilation).

---

## Notes

- The whisper-cpp binary must be present at `src-tauri/binaries/whisper-cpp-x86_64-unknown-linux-gnu` before building.
- The AppImage bundle target requires `linuxdeploy` (`paru -S linuxdeploy-bin`). It's not needed for this setup since we use the raw binary directly.
- Avoid `debtap` (`.deb` → Arch package conversion) for a dev workflow — it adds unnecessary steps on every rebuild.
