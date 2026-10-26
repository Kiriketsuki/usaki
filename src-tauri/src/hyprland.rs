//! Keeps the top-left corner of the bubble in place when its size changes on Hyprland.
//!
//! Hyprland keeps the centre of a floating window when the client changes its own size.
//! The bubble grows and shrinks with the account count, and it must keep its top-left
//! corner. So the app reads its position before a resize and moves the window back after
//! it. Every call matches the window by the process id, so it never touches another usaki
//! process.

use std::process::Command;
use std::time::Duration;

/// Hyprland applies the new client size a moment after the GTK call. The move waits this
/// long, so the re-centre does not undo it.
const SETTLE: Duration = Duration::from_millis(150);

/// Returns true when the app runs inside Hyprland.
pub fn is_hyprland() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

/// Returns the top-left corner of the window of process `pid` in `hyprctl -j clients` output.
pub fn position_of(clients_json: &str, pid: u32) -> Option<(i64, i64)> {
    let clients: Vec<serde_json::Value> = serde_json::from_str(clients_json).ok()?;
    let client = clients
        .iter()
        .find(|c| c["pid"].as_u64() == Some(u64::from(pid)))?;
    let at = client["at"].as_array()?;
    Some((at.first()?.as_i64()?, at.get(1)?.as_i64()?))
}

/// Returns the hyprctl arguments that move the window of process `pid` to (x, y).
pub fn move_args(pid: u32, x: i64, y: i64) -> [String; 3] {
    [
        "dispatch".to_owned(),
        "movewindowpixel".to_owned(),
        format!("exact {x} {y},pid:{pid}"),
    ]
}

/// Returns the top-left corner of this process's window, or None when hyprctl fails.
pub fn own_position() -> Option<(i64, i64)> {
    let out = Command::new("hyprctl")
        .args(["-j", "clients"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    position_of(&String::from_utf8_lossy(&out.stdout), std::process::id())
}

/// Moves this process's window back to `at` after SETTLE, on its own thread.
pub fn restore_position(at: (i64, i64)) {
    std::thread::spawn(move || {
        std::thread::sleep(SETTLE);
        let args = move_args(std::process::id(), at.0, at.1);
        match Command::new("hyprctl").args(&args).output() {
            Ok(out) if out.status.success() => {}
            Ok(out) => eprintln!(
                "usaki: hyprctl cannot move the bubble: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
            Err(e) => eprintln!("usaki: Cannot run hyprctl to move the bubble: {e}"),
        }
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

    #[test]
    fn moves_by_pid_and_never_by_class() {
        assert_eq!(
            move_args(746342, 2804, 463),
            ["dispatch", "movewindowpixel", "exact 2804 463,pid:746342"]
        );
    }
}
