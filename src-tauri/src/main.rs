//! usaKi: a floating bubble that shows the usage of each AI subscription.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod accounts;
#[cfg(target_os = "linux")]
mod hyprland;
mod poller;
mod view;
mod watch;
mod window;

use tauri::{Manager, State};

use crate::poller::CacheDir;
use crate::view::UsageUpdate;
use crate::watch::WatchRecords;
use crate::window::{BUBBLE_HEIGHT, BUBBLE_WIDTH};

/// The label of the bubble window in tauri.conf.json.
const BUBBLE_LABEL: &str = "bubble";

/// Returns the usage of every account. The page calls it once at start, before the
/// first `usage://update` event arrives.
#[tauri::command]
async fn get_usage(
    cache_dir: State<'_, CacheDir>,
    watched: State<'_, WatchRecords>,
) -> Result<UsageUpdate, String> {
    poller::poll_in_background(cache_dir.inner().clone(), watched.inner().clone()).await
}

/// Sets the bubble size. The page calls it when the cell count changes the cluster size.
/// On Hyprland the app then moves the window back, so the top-left corner stays put.
#[tauri::command]
fn set_bubble_size(window: tauri::WebviewWindow, width: i32, height: i32) -> Result<(), String> {
    let (width, height) = window::checked_size(width, height)?;
    #[cfg(target_os = "linux")]
    let before = hyprland::is_hyprland()
        .then(hyprland::own_position)
        .flatten();
    window::set_size(&window, width, height).map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    if let Some(at) = before {
        hyprland::restore_position(at);
    }
    Ok(())
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let bubble = app
        .get_webview_window(BUBBLE_LABEL)
        .ok_or("The bubble window does not exist.")?;
    // The config creates the window hidden, so the first map already has the bubble size.
    window::set_size(&bubble, BUBBLE_WIDTH, BUBBLE_HEIGHT)?;
    bubble.show()?;

    let cache_dir = CacheDir::from_env();
    if let Err(reason) = &cache_dir.0 {
        eprintln!("usaki: {reason}");
    }
    let watched = WatchRecords::default();
    app.manage(cache_dir.clone());
    app.manage(watched.clone());
    watch::spawn(app.handle().clone(), cache_dir.clone(), watched.clone());
    poller::spawn(app.handle().clone(), cache_dir, watched);
    Ok(())
}

fn main() {
    #[cfg(target_os = "linux")]
    window::prepare_linux();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_usage, set_bubble_size])
        .setup(setup)
        .run(tauri::generate_context!())
        .expect("usaki cannot start.");
}
