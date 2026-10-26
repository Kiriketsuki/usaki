//! usaKi: a floating bubble that shows the usage of each AI subscription.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod accounts;
mod poller;
mod view;
mod window;

use tauri::{Manager, State};

use crate::poller::CacheDir;
use crate::view::UsageUpdate;
use crate::window::{BUBBLE_HEIGHT, BUBBLE_WIDTH};

/// The label of the bubble window in tauri.conf.json.
const BUBBLE_LABEL: &str = "bubble";

/// Returns the usage of every account. The page calls it once at start, before the
/// first `usage://update` event arrives.
#[tauri::command]
async fn get_usage(cache_dir: State<'_, CacheDir>) -> Result<UsageUpdate, String> {
    poller::poll_in_background(cache_dir.inner().clone()).await
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
    app.manage(cache_dir.clone());
    poller::spawn(app.handle().clone(), cache_dir);
    Ok(())
}

fn main() {
    #[cfg(target_os = "linux")]
    window::prepare_linux();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_usage])
        .setup(setup)
        .run(tauri::generate_context!())
        .expect("usaki cannot start.");
}
