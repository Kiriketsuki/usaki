//! Window size and the Linux process setup that must run before Tauri starts.

use tauri::WebviewWindow;

/// The bubble size: the 152 px hex plus the 36 px ear band above it.
pub const BUBBLE_WIDTH: i32 = 152;
/// See `BUBBLE_WIDTH`.
pub const BUBBLE_HEIGHT: i32 = 188;

/// The Wayland app_id of the bubble. The Hyprland rule `usaki-bubble` matches it exactly.
#[cfg(target_os = "linux")]
pub const APP_ID: &str = "usaki";

/// Prepares the process environment on Linux. `main` must call it before Tauri starts.
#[cfg(target_os = "linux")]
pub fn prepare_linux() {
    // WebKitGTK draws nothing on the NVIDIA machine of the spike unless this variable is
    // set. See docs/spikes/2026-10-23-hyprland-window.md, "Environment variables". The
    // app keeps a value that the user sets.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // SAFETY: main calls this function before Tauri starts, so no other thread
        // reads or writes the environment at this time.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }
    // Tao creates its GTK application without an id, so GDK sends the GLib program
    // name as the Wayland app_id. That name is the binary file name unless the app
    // sets it. This call keeps the app_id `usaki` for a renamed or symlinked binary.
    gtk::glib::set_prgname(Some(APP_ID));
}

/// Sets the window size in logical pixels.
///
/// On Hyprland the Tauri `set_size` call returns `Ok` but does nothing. GTK
/// `set_size_request` plus `resize` works. See the spike report, "Sizing on Hyprland".
#[cfg(target_os = "linux")]
pub fn set_size(window: &WebviewWindow, width: i32, height: i32) -> tauri::Result<()> {
    use gtk::prelude::*;
    let gtk_window = window.gtk_window()?;
    gtk_window.set_size_request(width, height);
    gtk_window.resize(width, height);
    Ok(())
}

/// Sets the window size in logical pixels.
#[cfg(not(target_os = "linux"))]
pub fn set_size(window: &WebviewWindow, width: i32, height: i32) -> tauri::Result<()> {
    window.set_size(tauri::LogicalSize::new(f64::from(width), f64::from(height)))
}
