//! Keeps the bubble on its own screen on Hyprland.
//!
//! The bubble is a pinned window. When another monitor switches workspace, Hyprland can
//! move a pinned window onto that workspace, and so onto that monitor. A workspace script
//! that switches every monitor at once does this on each key press. The app listens to the
//! Hyprland event socket, and after a burst of workspace events it moves its window back to
//! the home monitor and the home corner. The move uses `movetoworkspacesilent` and then
//! `movewindowpixel`. That order keeps the pin. The app never toggles the pin, because the
//! spike found that a pin toggle can hide the window over a fullscreen window.

use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

use crate::hyprland::{self, Monitor};
use crate::placement::{Anchor, Home, Tracker};
use crate::position::SavedPosition;

/// The events after which a pinned window can sit on another monitor.
pub const TRIGGERS: &[&str] = &[
    "workspace",
    "workspacev2",
    "focusedmon",
    "focusedmonv2",
    "moveworkspace",
    "moveworkspacev2",
    "movewindow",
    "movewindowv2",
];

/// A workspace script sends a burst of events. The check runs once the burst has been quiet
/// this long.
const DEBOUNCE: Duration = Duration::from_millis(200);
/// The wait before the app connects again after the socket closes.
const RECONNECT: Duration = Duration::from_secs(2);
/// The time between two checks while a drag may still move the window.
const DRAG_POLL: Duration = Duration::from_millis(200);
/// The longest wait for a drag to settle before the check runs anyway.
const DRAG_WAIT: Duration = Duration::from_secs(30);
/// The longest wait for the first map and the restore move before the home is read.
const START_WAIT: Duration = Duration::from_secs(5);

/// Returns true when an event line of socket2, `name>>data`, is one of the TRIGGERS.
pub fn is_trigger(line: &str) -> bool {
    line.split_once(">>")
        .is_some_and(|(name, _)| TRIGGERS.contains(&name))
}

/// A move back to the home monitor: the workspace that the home monitor shows, and the home
/// corner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickMove {
    pub workspace: i64,
    pub x: i64,
    pub y: i64,
}

/// Where the window is now, from `hyprctl -j clients`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientAt {
    pub monitor: i64,
    pub workspace: i64,
}

/// Decides whether the window must move back, and where to. The window stays when it is on
/// the home monitor and on the workspace that the home monitor shows. A pinned window can
/// sit on the workspace of another monitor first, and that monitor takes it on its next
/// switch, so the workspace must match too. The window also stays when the home has no
/// monitor name, or when the home monitor is gone.
pub fn stick_move(
    home: &SavedPosition,
    client: Option<ClientAt>,
    monitors: &[Monitor],
) -> Option<StickMove> {
    let name = home.monitor.as_deref()?;
    let target = monitors
        .iter()
        .find(|m| m.rect.name.as_deref() == Some(name))?;
    let client = client?;
    if client.monitor == target.id && Some(client.workspace) == target.active_workspace {
        return None;
    }
    Some(StickMove {
        workspace: target.active_workspace?,
        x: home.x,
        y: home.y,
    })
}

/// Returns the path of the Hyprland event socket.
fn socket_path() -> Option<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
    let instance = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    Some(
        PathBuf::from(runtime)
            .join("hypr")
            .join(instance)
            .join(".socket2.sock"),
    )
}

/// Starts the event reader and the check worker. It does nothing outside Hyprland.
pub fn spawn(app: AppHandle) {
    let Some(path) = socket_path() else {
        return;
    };
    let (tx, rx) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        loop {
            match UnixStream::connect(&path) {
                Ok(stream) => {
                    for line in BufReader::new(stream).lines() {
                        let Ok(line) = line else { break };
                        if is_trigger(&line) && tx.send(()).is_err() {
                            return;
                        }
                    }
                    eprintln!("usaki: The Hyprland event socket closed. The app connects again.");
                }
                Err(e) => eprintln!(
                    "usaki: Cannot connect to the Hyprland event socket {}. {e}",
                    path.display()
                ),
            }
            std::thread::sleep(RECONNECT);
        }
    });
    std::thread::spawn(move || {
        init_home(&app);
        // A pinned window maps on the focused monitor, and `movewindowpixel` alone does not
        // change its monitor. So the restore to another screen needs this first check.
        check(&app);
        while rx.recv().is_ok() {
            // Wait until the burst is quiet.
            loop {
                match rx.recv_timeout(DEBOUNCE) {
                    Ok(()) => {}
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
            wait_for_drag(&app);
            check(&app);
        }
    });
}

/// Sets the home from the window after the first map, unless the restore already set it.
fn init_home(app: &AppHandle) {
    let start = Instant::now();
    let anchor = app.state::<Anchor>();
    while (hyprland::own_position().is_none() || anchor.get().is_some())
        && start.elapsed() < START_WAIT
    {
        std::thread::sleep(DRAG_POLL);
    }
    if let Some(pos) = hyprland::own_placement() {
        app.state::<Home>().init(pos);
    }
}

/// A drag to another screen is the user's choice. The check waits until the tracker has
/// saved the new home, so it does not pull the window back.
fn wait_for_drag(app: &AppHandle) {
    let start = Instant::now();
    let tracker = app.state::<Tracker>();
    while tracker.active() && start.elapsed() < DRAG_WAIT {
        std::thread::sleep(DRAG_POLL);
    }
}

fn check(app: &AppHandle) {
    let Some(home) = app.state::<Home>().get() else {
        return;
    };
    let (Some(clients), Some(monitors)) = (
        hyprland::hyprctl_json("clients"),
        hyprland::hyprctl_json("monitors"),
    ) else {
        return;
    };
    let pid = std::process::id();
    let client = hyprland::monitor_id_of(&clients, pid)
        .zip(hyprland::workspace_id_of(&clients, pid))
        .map(|(monitor, workspace)| ClientAt { monitor, workspace });
    let Some(back) = stick_move(&home, client, &hyprland::monitors_of(&monitors)) else {
        return;
    };
    eprintln!(
        "usaki: Hyprland moved the bubble off {} or its workspace. The app moves it back to workspace {} at {},{}.",
        home.monitor.as_deref().unwrap_or("its monitor"),
        back.workspace,
        back.x,
        back.y
    );
    // A resize in this time must keep the home corner, not the corner on the wrong screen.
    let anchor = app.state::<Anchor>();
    anchor.set((back.x, back.y));
    hyprland::dispatch(&[
        "dispatch".to_owned(),
        "movetoworkspacesilent".to_owned(),
        format!("{},pid:{pid}", back.workspace),
    ]);
    hyprland::dispatch(&hyprland::move_args(pid, back.x, back.y));
    anchor.clear_if((back.x, back.y));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::MonitorRect;

    fn monitor(id: i64, name: &str, x: i64, ws: i64) -> Monitor {
        Monitor {
            id,
            rect: MonitorRect {
                name: Some(name.to_owned()),
                x,
                y: 0,
                width: 1920,
                height: 1080,
            },
            active_workspace: Some(ws),
        }
    }

    // The monitors of the bug report after the workspace script ran with base 10.
    fn monitors() -> Vec<Monitor> {
        vec![
            monitor(0, "DP-2", 1920, 20),
            monitor(1, "DP-3", 3840, 30),
            monitor(2, "eDP-1", 0, 10),
        ]
    }

    fn home() -> SavedPosition {
        SavedPosition::new(4822, 531, Some("DP-3".into()))
    }

    fn at(monitor: i64, workspace: i64) -> Option<ClientAt> {
        Some(ClientAt { monitor, workspace })
    }

    #[test]
    fn moves_back_to_the_workspace_that_the_home_monitor_shows() {
        assert_eq!(
            stick_move(&home(), at(0, 20), &monitors()),
            Some(StickMove {
                workspace: 30,
                x: 4822,
                y: 531
            })
        );
    }

    #[test]
    fn stays_on_the_home_monitor_and_its_workspace() {
        assert_eq!(stick_move(&home(), at(1, 30), &monitors()), None);
    }

    #[test]
    fn moves_back_from_the_workspace_of_another_monitor() {
        // The live run saw this: monitor DP-3, but workspace 20, which DP-2 shows.
        assert_eq!(
            stick_move(&home(), at(1, 20), &monitors()).map(|m| m.workspace),
            Some(30)
        );
    }

    #[test]
    fn stays_when_the_home_has_no_monitor_or_the_monitor_is_gone() {
        let unnamed = SavedPosition::new(4822, 531, None);
        assert_eq!(stick_move(&unnamed, at(0, 20), &monitors()), None);
        let gone = SavedPosition::new(4822, 531, Some("HDMI-A-1".into()));
        assert_eq!(stick_move(&gone, at(0, 20), &monitors()), None);
    }

    #[test]
    fn stays_when_the_window_or_the_workspace_is_unknown() {
        assert_eq!(stick_move(&home(), None, &monitors()), None);
        let mut blank = monitors();
        blank[1].active_workspace = None;
        assert_eq!(stick_move(&home(), at(1, 30), &blank), None);
    }

    #[test]
    fn a_drag_to_another_screen_moves_the_home() {
        // The tracker saved the new home on DP-2 after the drag, so DP-2 is now right.
        let dragged = SavedPosition::new(2902, 531, Some("DP-2".into()));
        assert_eq!(stick_move(&dragged, at(0, 20), &monitors()), None);
    }

    #[test]
    fn reacts_only_to_workspace_and_move_events() {
        assert!(is_trigger("workspacev2>>20,20"));
        assert!(is_trigger("focusedmon>>DP-2,20"));
        assert!(is_trigger("moveworkspacev2>>20,20,DP-2"));
        assert!(is_trigger("movewindowv2>>55d0,20,20"));
        assert!(!is_trigger("activewindow>>kitty,zsh"));
        assert!(!is_trigger("openwindow>>55d0,20,usaki,usaki"));
        assert!(!is_trigger("workspace"));
    }
}
