//! Spike for issue #2. It tests a Tauri v2 window on Hyprland.
//!
//! The spike reads these environment variables:
//! - `USAKI_SPIKE_MODE`: `toplevel` (default) or `layer`. `layer` turns the window into a
//!   gtk-layer-shell overlay surface before it maps.
//! - `USAKI_SPIKE_NO_RESHOW=1`: skips the hide and show workaround in layer mode.
//! - `USAKI_SPIKE_GTK_SHOW=1`: maps the layer window with a bare gtk `show_all()`.
//! - `USAKI_SPIKE_OPAQUE_TEST=1`: diagnostic. GTK paints an opaque layer background.
//! - `USAKI_SPIKE_LAYER_MARGIN=L,T`: the first left and top margin of the layer surface.
//! - `USAKI_SPIKE_CMD_FILE=PATH`: the backend polls this file and runs each new line once.
//!   The commands are `geom`, `pos X Y`, `size W H`, `resizable 0|1`, `autotest`,
//!   `layer-move DX DY`, `gtk-size W H`, `gtk-reshow` and `quit`. `size` uses the Tauri
//!   API. `gtk-size` calls `set_size_request` and `resize` on the GTK window.
//!   `autotest` runs JS that calls `startDragging()` and a call that the capability
//!   does not grant. `layer-move` uses the same code path as a pointer drag in layer
//!   mode. All results go to stdout.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow};

/// Holds the current layer-shell margins. Only layer mode uses it.
struct LayerMargins(Mutex<(i32, i32)>);

fn stamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("[spike {now}]")
}

fn parse_pair(name: &str) -> Option<(i32, i32)> {
    let raw = std::env::var(name).ok()?;
    let (a, b) = raw.split_once(',')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

fn mode() -> String {
    std::env::var("USAKI_SPIKE_MODE").unwrap_or_else(|_| "toplevel".into())
}

#[tauri::command]
fn spike_log(msg: String) {
    println!("{} js: {msg}", stamp());
}

#[tauri::command]
fn spike_mode() -> String {
    mode()
}

/// Moves the layer surface by a pointer delta. A layer surface cannot use
/// `startDragging`, so a drag in layer mode changes the margins instead.
#[tauri::command]
fn layer_move_by(app: AppHandle, dx: i32, dy: i32) -> Result<(i32, i32), String> {
    let state = app.state::<LayerMargins>();
    let next = {
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        let next = ((guard.0 + dx).max(0), (guard.1 + dy).max(0));
        *guard = next;
        next
    };
    let window = app
        .get_webview_window("main")
        .ok_or("the main window does not exist")?;
    apply_layer_margins(&window, next)?;
    Ok(next)
}

#[cfg(target_os = "linux")]
fn apply_layer_margins(window: &WebviewWindow, (left, top): (i32, i32)) -> Result<(), String> {
    use gtk_layer_shell::{Edge, LayerShell};
    let gtk_window = window.gtk_window().map_err(|e| e.to_string())?;
    gtk_window.set_layer_shell_margin(Edge::Left, left);
    gtk_window.set_layer_shell_margin(Edge::Top, top);
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn apply_layer_margins(_window: &WebviewWindow, _m: (i32, i32)) -> Result<(), String> {
    Err("layer-shell exists only on Linux".into())
}

/// Turns the GTK window into a layer-shell overlay surface. The window must not be
/// mapped yet, so the config creates it with `visible: false`.
#[cfg(target_os = "linux")]
fn init_layer(window: &WebviewWindow, margins: (i32, i32)) -> Result<(), String> {
    use gtk::prelude::*;
    use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

    let gtk_window = window.gtk_window().map_err(|e| e.to_string())?;
    println!(
        "{} layer: before init realized={} visible={}",
        stamp(),
        gtk_window.is_realized(),
        gtk_window.is_visible()
    );
    if gtk_window.is_realized() {
        // gtk-layer-shell must run before the GDK window exists. Tao may realize
        // the window early, so the spike unrealizes it first.
        gtk_window.unrealize();
    }
    gtk_window.init_layer_shell();
    gtk_window.set_namespace("usaki");
    gtk_window.set_layer(Layer::Overlay);
    gtk_window.set_anchor(Edge::Top, true);
    gtk_window.set_anchor(Edge::Left, true);
    gtk_window.set_keyboard_mode(KeyboardMode::None);
    gtk_window.set_exclusive_zone(-1);
    if std::env::var("USAKI_SPIKE_OPAQUE_TEST").as_deref() == Ok("1") {
        // Diagnostic only: GTK then paints its default opaque background, which
        // shows whether the compositor displays the surface at all.
        gtk_window.set_app_paintable(false);
    }
    apply_layer_margins(window, margins)?;
    println!(
        "{} layer: init done is_layer_window={}",
        stamp(),
        gtk_window.is_layer_window()
    );
    // Tao tracks visibility itself, so the spike shows the window through Tauri.
    // A bare gtk show_all() maps the surface, but the webview then draws nothing.
    if std::env::var("USAKI_SPIKE_GTK_SHOW").as_deref() == Ok("1") {
        gtk_window.show_all();
    } else {
        window.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn init_layer(_window: &WebviewWindow, _m: (i32, i32)) -> Result<(), String> {
    Err("layer-shell exists only on Linux".into())
}

/// Runs a closure on the main thread after a delay.
fn after(app: &AppHandle, delay: Duration, f: impl FnOnce(AppHandle) + Send + 'static) {
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(delay);
        let inner = handle.clone();
        if let Err(e) = handle.run_on_main_thread(move || f(inner)) {
            eprintln!("{} run_on_main_thread failed: {e}", stamp());
        }
    });
}

const AUTOTEST_JS: &str = r#"
(async () => {
  const log = (m) => window.__TAURI__.core.invoke('spike_log', { msg: m });
  const win = window.__TAURI__.window.getCurrentWindow();
  try {
    await win.startDragging();
    await log('autotest startDragging resolved: no permission error');
  } catch (e) {
    await log('autotest startDragging rejected: ' + e);
  }
  try {
    await win.setPosition(new window.__TAURI__.dpi.PhysicalPosition(10, 10));
    await log('autotest setPosition resolved (control call)');
  } catch (e) {
    await log('autotest setPosition rejected (control call, expected): ' + e);
  }
})();
"#;

/// Runs one spike command on the main thread. The commands let the test driver change
/// the window while the app runs, without a pointer or a keyboard.
fn run_command(app: &AppHandle, line: &str) {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let Some(w) = app.get_webview_window("main") else {
        println!("{} cmd: the main window does not exist", stamp());
        return;
    };
    let num = |i: usize| {
        parts
            .get(i)
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0)
    };
    let out = match parts.first().copied() {
        Some("geom") => format!(
            "inner_size={:?} outer_size={:?} outer_position={:?}",
            w.inner_size(),
            w.outer_size(),
            w.outer_position()
        ),
        Some("pos") => format!(
            "set_position({},{}) -> {:?}",
            num(1),
            num(2),
            w.set_position(PhysicalPosition::new(num(1), num(2)))
        ),
        Some("size") => format!(
            "set_size({},{}) -> {:?}",
            num(1),
            num(2),
            w.set_size(tauri::PhysicalSize::new(
                num(1).max(1) as u32,
                num(2).max(1) as u32
            ))
        ),
        Some("resizable") => format!(
            "set_resizable({}) -> {:?}",
            num(1) == 1,
            w.set_resizable(num(1) == 1)
        ),
        Some("autotest") => format!("eval -> {:?}", w.eval(AUTOTEST_JS)),
        Some("layer-move") => format!(
            "layer_move_by -> {:?}",
            layer_move_by(app.clone(), num(1), num(2))
        ),
        #[cfg(target_os = "linux")]
        Some("gtk-reshow") => {
            use gtk::prelude::*;
            match w.gtk_window() {
                Ok(g) => {
                    g.hide();
                    g.show_all();
                    g.queue_draw();
                    format!("realized={} mapped={}", g.is_realized(), g.is_mapped())
                }
                Err(e) => e.to_string(),
            }
        }
        #[cfg(target_os = "linux")]
        Some("gtk-size") => {
            use gtk::prelude::*;
            match w.gtk_window() {
                Ok(g) => {
                    g.set_size_request(num(1), num(2));
                    g.resize(num(1), num(2));
                    format!("gtk set_size_request and resize({},{})", num(1), num(2))
                }
                Err(e) => e.to_string(),
            }
        }
        Some("quit") => {
            app.exit(0);
            "exit".into()
        }
        _ => "unknown command".into(),
    };
    println!("{} cmd '{line}': {out}", stamp());
}

/// Polls a command file and runs each new line once.
fn watch_commands(app: AppHandle, path: String) {
    std::thread::spawn(move || {
        let mut done = 0usize;
        loop {
            std::thread::sleep(Duration::from_millis(200));
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let lines: Vec<String> = text.lines().map(str::to_owned).collect();
            for line in lines.iter().skip(done) {
                let inner = app.clone();
                let cmd = line.clone();
                let _ = app.run_on_main_thread(move || run_command(&inner, &cmd));
            }
            done = lines.len();
        }
    });
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("main")
        .ok_or("the main window does not exist")?;
    let margins = parse_pair("USAKI_SPIKE_LAYER_MARGIN").unwrap_or((40, 40));
    app.manage(LayerMargins(Mutex::new(margins)));
    println!("{} setup: mode={}", stamp(), mode());

    if mode() == "layer" {
        init_layer(&window, margins)?;
    } else {
        window.show()?;
    }

    let handle = app.handle().clone();
    if mode() == "layer" && std::env::var("USAKI_SPIKE_NO_RESHOW").as_deref() != Ok("1") {
        // Workaround: on the first map the layer surface acks its configure but GTK
        // never attaches a buffer, so the surface stays blank. One hide and show
        // after the map makes GTK draw. See the spike report, check 7.
        after(&handle, Duration::from_millis(300), |app| {
            run_command(&app, "gtk-reshow")
        });
    }
    after(&handle, Duration::from_millis(1500), |app| {
        run_command(&app, "geom")
    });
    if let Ok(path) = std::env::var("USAKI_SPIKE_CMD_FILE") {
        watch_commands(handle, path);
    }
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            spike_log,
            spike_mode,
            layer_move_by
        ])
        .setup(setup)
        .run(tauri::generate_context!())
        .expect("the spike failed to start");
}
