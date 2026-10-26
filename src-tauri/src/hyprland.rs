//! Window position on Hyprland. Tauri `set_position` does nothing there, so the app reads
//! and moves its window through `hyprctl`.
//!
//! Hyprland keeps the centre of a floating window when the client changes its own size.
//! The bubble grows and shrinks with the account count and with the card, and it must keep
//! its top-left corner. So the app reads its position before a resize and moves the window
//! back after it. The same move restores the saved position after the first map. Every call
//! matches the window by the process id, so it never touches another usaki process.

use std::process::Command;
use std::time::{Duration, Instant};

use crate::placement::Anchor;
use crate::position::{MonitorRect, SavedPosition, monitor_at};

/// Hyprland applies the new client size a moment after the GTK call. The move waits this
/// long, so the re-centre does not undo it.
const SETTLE: Duration = Duration::from_millis(150);
/// The longest wait for the first map before a move gives up.
const MAP_WAIT: Duration = Duration::from_secs(3);
/// The time between two checks for the map.
const MAP_POLL: Duration = Duration::from_millis(50);

/// Returns true when the app runs inside Hyprland.
pub fn is_hyprland() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

fn client_of(clients_json: &str, pid: u32) -> Option<serde_json::Value> {
    let clients: Vec<serde_json::Value> = serde_json::from_str(clients_json).ok()?;
    clients
        .into_iter()
        .find(|c| c["pid"].as_u64() == Some(u64::from(pid)))
}

/// Returns the top-left corner of the window of process `pid` in `hyprctl -j clients` output.
pub fn position_of(clients_json: &str, pid: u32) -> Option<(i64, i64)> {
    let client = client_of(clients_json, pid)?;
    let at = client["at"].as_array()?;
    Some((at.first()?.as_i64()?, at.get(1)?.as_i64()?))
}

/// Returns the monitor id of the window of process `pid` in `hyprctl -j clients` output.
pub fn monitor_id_of(clients_json: &str, pid: u32) -> Option<i64> {
    client_of(clients_json, pid)?["monitor"].as_i64()
}

/// One monitor of `hyprctl -j monitors`: its id and its rectangle in layout coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Monitor {
    pub id: i64,
    pub rect: MonitorRect,
}

/// Returns the monitors in `hyprctl -j monitors` output. The width and the height there are
/// in physical pixels before the rotation, so the function divides by the scale and swaps
/// them for a 90 or 270 degree transform. A monitor with a missing field is skipped.
pub fn monitors_of(monitors_json: &str) -> Vec<Monitor> {
    let list: Vec<serde_json::Value> = serde_json::from_str(monitors_json).unwrap_or_default();
    list.iter()
        .filter_map(|m| {
            let scale = m["scale"].as_f64().filter(|s| *s > 0.0).unwrap_or(1.0);
            let logical = |key: &str| m[key].as_f64().map(|v| (v / scale).round() as i64);
            let (mut width, mut height) = (logical("width")?, logical("height")?);
            if m["transform"].as_i64().unwrap_or(0) % 2 == 1 {
                std::mem::swap(&mut width, &mut height);
            }
            Some(Monitor {
                id: m["id"].as_i64()?,
                rect: MonitorRect {
                    name: m["name"].as_str().map(str::to_owned),
                    x: m["x"].as_i64()?,
                    y: m["y"].as_i64()?,
                    width,
                    height,
                },
            })
        })
        .collect()
}

/// Returns the placement of the window of process `pid` from the hyprctl outputs. The
/// monitor comes from the client's monitor id, else from the monitor under the corner.
pub fn placement_of(clients_json: &str, monitors_json: &str, pid: u32) -> Option<SavedPosition> {
    let (x, y) = position_of(clients_json, pid)?;
    let monitors = monitors_of(monitors_json);
    let by_id = monitor_id_of(clients_json, pid)
        .and_then(|id| monitors.iter().find(|m| m.id == id))
        .and_then(|m| m.rect.name.clone());
    let rects: Vec<MonitorRect> = monitors.into_iter().map(|m| m.rect).collect();
    let name = by_id.or_else(|| monitor_at(&rects, x, y));
    Some(SavedPosition::new(x, y, name))
}

fn hyprctl_json(what: &str) -> Option<String> {
    let out = Command::new("hyprctl").args(["-j", what]).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Returns the hyprctl arguments that move the window of process `pid` to (x, y).
pub fn move_args(pid: u32, x: i64, y: i64) -> [String; 3] {
    [
        "dispatch".to_owned(),
        "movewindowpixel".to_owned(),
        format!("exact {x} {y},pid:{pid}"),
    ]
}

/// Returns the top-left corner of this process's window, or None when hyprctl fails or the
/// window is not mapped yet.
pub fn own_position() -> Option<(i64, i64)> {
    position_of(&hyprctl_json("clients")?, std::process::id())
}

/// Returns the placement of this process's window, or None when hyprctl fails.
pub fn own_placement() -> Option<SavedPosition> {
    placement_of(
        &hyprctl_json("clients")?,
        &hyprctl_json("monitors")?,
        std::process::id(),
    )
}

/// Returns the monitors, or no monitor when hyprctl fails.
pub fn monitor_rects() -> Vec<MonitorRect> {
    hyprctl_json("monitors")
        .map(|json| monitors_of(&json).into_iter().map(|m| m.rect).collect())
        .unwrap_or_default()
}

/// Moves this process's window to `at` after SETTLE, on its own thread. It first waits for
/// the map, so the same call restores the saved position at start. The anchor holds `at`
/// until the move is done, so a resize in that time keeps `at` and not the old corner.
pub fn move_after_settle(at: (i64, i64), anchor: Anchor) {
    anchor.set(at);
    std::thread::spawn(move || {
        std::thread::sleep(SETTLE);
        let start = Instant::now();
        while own_position().is_none() && start.elapsed() < MAP_WAIT {
            std::thread::sleep(MAP_POLL);
        }
        let args = move_args(std::process::id(), at.0, at.1);
        match Command::new("hyprctl").args(&args).output() {
            Ok(out) if out.status.success() => {}
            Ok(out) => eprintln!(
                "usaki: hyprctl cannot move the bubble: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
            Err(e) => eprintln!("usaki: Cannot run hyprctl to move the bubble: {e}"),
        }
        anchor.clear_if(at);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLIENTS: &str = r#"[
        {"pid": 100, "at": [10, 20], "class": "usaki"},
        {"pid": 200, "at": [1494, 490], "class": "usaki"}
    ]"#;

    #[test]
    fn finds_the_window_of_the_pid() {
        assert_eq!(position_of(CLIENTS, 200), Some((1494, 490)));
        assert_eq!(position_of(CLIENTS, 100), Some((10, 20)));
    }

    #[test]
    fn gives_none_for_a_missing_pid_or_bad_json() {
        assert_eq!(position_of(CLIENTS, 300), None);
        assert_eq!(position_of("not json", 100), None);
        assert_eq!(position_of(r#"[{"pid": 100, "at": [1]}]"#, 100), None);
    }

    const MONITORS: &str = r#"[
        {"id": 0, "name": "DP-2", "x": 1920, "y": 0, "width": 1920, "height": 1080, "scale": 1.0, "transform": 0},
        {"id": 1, "name": "DP-3", "x": 3840, "y": 0, "width": 2560, "height": 1080, "scale": 1.0, "transform": 0},
        {"id": 2, "name": "eDP-1", "x": 0, "y": 0, "width": 2880, "height": 1800, "scale": 1.5, "transform": 1}
    ]"#;

    #[test]
    fn reads_the_monitors_in_layout_coordinates() {
        let found = monitors_of(MONITORS);
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].id, 0);
        assert_eq!(found[0].rect.name.as_deref(), Some("DP-2"));
        assert_eq!((found[0].rect.x, found[0].rect.width), (1920, 1920));
        // 2880x1800 at scale 1.5 is 1920x1200. A 90 degree turn swaps the two.
        assert_eq!((found[2].rect.width, found[2].rect.height), (1200, 1920));
        assert!(monitors_of("not json").is_empty());
    }

    #[test]
    fn places_the_window_on_its_client_monitor() {
        let clients = r#"[{"pid": 7, "at": [2804, 463], "monitor": 0}]"#;
        assert_eq!(
            placement_of(clients, MONITORS, 7),
            Some(SavedPosition::new(2804, 463, Some("DP-2".into())))
        );
    }

    #[test]
    fn names_the_monitor_by_the_corner_when_the_id_is_unknown() {
        let clients = r#"[{"pid": 7, "at": [4000, 10], "monitor": 9}]"#;
        assert_eq!(
            placement_of(clients, MONITORS, 7),
            Some(SavedPosition::new(4000, 10, Some("DP-3".into())))
        );
        assert_eq!(placement_of(clients, MONITORS, 8), None);
    }

    #[test]
    fn moves_by_pid_and_never_by_class() {
        assert_eq!(
            move_args(746342, 2804, 463),
            ["dispatch", "movewindowpixel", "exact 2804 463,pid:746342"]
        );
    }
}
