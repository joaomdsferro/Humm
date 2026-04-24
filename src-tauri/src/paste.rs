pub fn paste_text(text: &str) -> Result<(), String> {
    // Set clipboard (arboard is thread-safe)
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())?;

    // Small delay to ensure clipboard is set
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Simulate Cmd+V via osascript (works from any thread, unlike enigo which
    // calls TSMGetInputSourceProperty requiring the main thread)
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("osascript")
            .args(["-e", r#"tell application "System Events" to keystroke "v" using command down"#])
            .output()
            .map_err(|e| format!("Failed to simulate paste: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        use enigo::{Enigo, Keyboard, Settings, Key, Direction};
        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        enigo.key(Key::Control, Direction::Press).map_err(|e| e.to_string())?;
        enigo.key(Key::Unicode('v'), Direction::Click).map_err(|e| e.to_string())?;
        enigo.key(Key::Control, Direction::Release).map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        let uid = unsafe { libc::getuid() };
        let runtime_dir = format!("/run/user/{}", uid);
        let wayland_display = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".to_string());

        // wl-copy serves clipboard requests until a paste occurs; spawn it in
        // the background so it stays alive to answer the upcoming Ctrl+V.
        let _wl = std::process::Command::new("wl-copy")
            .env("WAYLAND_DISPLAY", &wayland_display)
            .env("XDG_RUNTIME_DIR", &runtime_dir)
            .arg("--")
            .arg(text)
            .spawn()
            .map_err(|e| format!("wl-copy failed: {}", e))?;

        // Give wl-copy time to register as clipboard owner.
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Try wtype first — Wayland-native, no daemon required.
        let wtype_ok = std::process::Command::new("wtype")
            .env("WAYLAND_DISPLAY", &wayland_display)
            .env("XDG_RUNTIME_DIR", &runtime_dir)
            .args(["-M", "ctrl", "v", "-m", "ctrl"])
            .output()
            .map(|r| r.status.success())
            .unwrap_or(false);
        if wtype_ok {
            return Ok(());
        }

        // Fall back to ydotool (requires ydotoold daemon).
        let socket = format!("{}/.ydotool_socket", runtime_dir);
        // Release any modifier keys still registered from the hotkey chord.
        let _ = std::process::Command::new("ydotool")
            .env("YDOTOOL_SOCKET", &socket)
            .args(["key", "29:0", "42:0", "54:0", "57:0"])
            .output();
        std::thread::sleep(std::time::Duration::from_millis(50));

        let yd = std::process::Command::new("ydotool")
            .env("YDOTOOL_SOCKET", &socket)
            .args(["key", "29:1", "47:1", "47:0", "29:0"])
            .output()
            .map_err(|e| format!("ydotool failed: {}", e))?;
        if !yd.status.success() {
            return Err(format!("ydotool error: {}", String::from_utf8_lossy(&yd.stderr)));
        }

        return Ok(());
    }

    #[allow(unreachable_code)]
    Ok(())
}
