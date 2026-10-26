//! usaKi: a floating bubble that shows the usage of each AI subscription.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod accounts;
mod codex;
#[cfg(target_os = "linux")]
mod hyprland;
mod placement;
mod poller;
mod position;
#[cfg(target_os = "linux")]
mod stick;
mod view;
mod watch;
mod window;

use tauri::{Manager, State, WindowEvent};

use crate::codex::CodexSources;
use crate::placement::{Anchor, Home, Tracker};
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
    codex: State<'_, CodexSources>,
) -> Result<UsageUpdate, String> {
    poller::poll_in_background(
        cache_dir.inner().clone(),
        watched.inner().clone(),
        codex.inner().clone(),
    )
    .await
}

/// Sets the window size. The page calls it when the cell count changes the cluster size,
/// and when the card opens or closes. On Hyprland the app then moves the window back, so
/// the top-left corner stays put. A pending move gives the corner to keep.
#[tauri::command]
fn set_bubble_size(
    window: tauri::WebviewWindow,
    anchor: State<'_, Anchor>,
    width: i32,
    height: i32,
) -> Result<(), String> {
    let (width, height) = window::checked_size(width, height)?;
    #[cfg(target_os = "linux")]
    let before = hyprland::is_hyprland()
        .then(|| anchor.get().or_else(hyprland::own_position))
        .flatten();
    #[cfg(not(target_os = "linux"))]
    let _ = anchor;
    window::set_size(&window, width, height).map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    if let Some(at) = before {
        hyprland::move_after_settle(at, anchor.inner().clone());
    }
    Ok(())
}

/// The environment variable that holds the debug script of mouse actions. See ui/src/debug.ts.
const DEBUG_UI_ENV: &str = "USAKI_DEBUG_UI";

/// Returns the debug script in a debug build. A release build always returns None, so the
/// variable has no effect there.
#[tauri::command]
fn debug_ui() -> Option<String> {
    if cfg!(debug_assertions) {
        std::env::var(DEBUG_UI_ENV).ok()
    } else {
        None
    }
}

/// Watches the window position and saves it when it settles. The page calls it when a drag
/// starts and when it sees the drag end.
#[tauri::command]
fn track_position(window: tauri::WebviewWindow, tracker: State<'_, Tracker>) {
    placement::track(&window, &tracker);
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let bubble = app
        .get_webview_window(BUBBLE_LABEL)
        .ok_or("The bubble window does not exist.")?;
    // The config creates the window hidden, so the first map already has the bubble size.
    window::set_size(&bubble, BUBBLE_WIDTH, BUBBLE_HEIGHT)?;
    let anchor = Anchor::default();
    let home = Home::default();
    placement::restore(&bubble, &anchor, &home);
    app.manage(anchor);
    app.manage(home);
    app.manage(Tracker::default());
    #[cfg(target_os = "linux")]
    if hyprland::is_hyprland() {
        stick::spawn(app.handle().clone());
    }
    bubble.show()?;
    // A close request is the last time the window still has a position to read.
    let closing = bubble.clone();
    bubble.on_window_event(move |event| {
        if matches!(event, WindowEvent::CloseRequested { .. })
            && let Err(reason) = placement::save(&closing)
        {
            eprintln!("usaki: {reason}");
        }
    });

    let cache_dir = CacheDir::from_env();
    if let Err(reason) = &cache_dir.0 {
        eprintln!("usaki: {reason}");
    }
    let watched = WatchRecords::default();
    let codex = CodexSources::from_env();
    app.manage(cache_dir.clone());
    app.manage(watched.clone());
    app.manage(codex.clone());
    watch::spawn(
        app.handle().clone(),
        cache_dir.clone(),
        watched.clone(),
        codex.clone(),
    );
    poller::spawn(app.handle().clone(), cache_dir, watched, codex);
    Ok(())
}

fn main() {
    #[cfg(target_os = "linux")]
    window::prepare_linux();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_usage,
            set_bubble_size,
            track_position,
            debug_ui
        ])
        .setup(setup)
        .run(tauri::generate_context!())
        .expect("usaki cannot start.");
}
