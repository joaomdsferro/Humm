//! Capture text the user wants read aloud.
//!
//! Strategy:
//! - Linux: try Wayland/X11 primary selection (auto-updated on highlight),
//!   fall back to the regular clipboard.
//! - macOS/Windows: read the regular clipboard (user must Ctrl+C first;
//!   there is no system-wide notion of "current selection" we can read
//!   without accessibility APIs).

pub fn capture_text() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        if let Some(text) = try_primary_selection() {
            if !text.trim().is_empty() {
                return Ok(text);
            }
        }
    }

    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let text = clipboard
        .get_text()
        .map_err(|e| format!("Clipboard empty or unreadable: {}", e))?;
    if text.trim().is_empty() {
        return Err("No text in selection or clipboard".to_string());
    }
    Ok(text)
}

#[cfg(target_os = "linux")]
fn try_primary_selection() -> Option<String> {
    let uid = unsafe { libc::getuid() };
    let runtime_dir = format!("/run/user/{}", uid);
    let wayland_display =
        std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".to_string());

    // Wayland: wl-paste --primary
    if let Ok(out) = std::process::Command::new("wl-paste")
        .env("WAYLAND_DISPLAY", &wayland_display)
        .env("XDG_RUNTIME_DIR", &runtime_dir)
        .args(["--primary", "--no-newline"])
        .output()
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).to_string();
            if !s.trim().is_empty() {
                return Some(s);
            }
        }
    }

    // X11: xclip -selection primary -o
    if let Ok(out) = std::process::Command::new("xclip")
        .args(["-selection", "primary", "-o"])
        .output()
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).to_string();
            if !s.trim().is_empty() {
                return Some(s);
            }
        }
    }

    // X11 fallback: xsel -p
    if let Ok(out) = std::process::Command::new("xsel").args(["-p"]).output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).to_string();
            if !s.trim().is_empty() {
                return Some(s);
            }
        }
    }

    None
}
