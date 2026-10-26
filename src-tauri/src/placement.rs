//! Reads, saves and restores the window placement on each platform. The file format and
//! the validity check are in `position.rs`.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use tauri::{Manager, WebviewWindow};

use crate::position::{self, MonitorRect, SavedPosition};

/// The top-left corner that a pending move will set. A resize in that time must keep this
/// corner, because the window has not reached it yet. Only Hyprland sets it.
#[derive(Debug, Clone, Default)]
pub struct Anchor(Arc<Mutex<Option<(i64, i64)>>>);

impl Anchor {
    pub fn get(&self) -> Option<(i64, i64)> {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn set(&self, at: (i64, i64)) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(at);
    }

    /// Clears the anchor when it still holds `at`. A newer move keeps its own corner.
    pub fn clear_if(&self, at: (i64, i64)) {
        let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if *slot == Some(at) {
            *slot = None;
        }
    }
}

/// The placement that the window belongs at: the saved or restored one, or the one at
/// start. A drag that settles replaces it, so a drag to another screen moves the home too.
#[derive(Debug, Clone, Default)]
pub struct Home(Arc<Mutex<Option<SavedPosition>>>);

impl Home {
    pub fn get(&self) -> Option<SavedPosition> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn set(&self, pos: SavedPosition) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(pos);
    }

    /// Sets the home only when none is set yet.
    pub fn init(&self, pos: SavedPosition) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_or_insert(pos);
    }
}

#[cfg(target_os = "linux")]
fn on_hyprland() -> bool {
    crate::hyprland::is_hyprland()
}

#[cfg(not(target_os = "linux"))]
fn on_hyprland() -> bool {
    false
}

/// Returns the current placement of the window, or None when the platform cannot tell.
pub fn current(window: &WebviewWindow) -> Option<SavedPosition> {
    #[cfg(target_os = "linux")]
    if on_hyprland() {
        return crate::hyprland::own_placement();
    }
    let at = window.outer_position().ok()?;
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .and_then(|m| m.name().cloned());
    Some(SavedPosition::new(at.x.into(), at.y.into(), monitor))
}

/// Returns the monitors in the same coordinates as `current`.
fn monitors(window: &WebviewWindow) -> Vec<MonitorRect> {
    #[cfg(target_os = "linux")]
    if on_hyprland() {
        return crate::hyprland::monitor_rects();
    }
    window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| MonitorRect {
            name: m.name().cloned(),
            x: m.position().x.into(),
            y: m.position().y.into(),
            width: m.size().width.into(),
            height: m.size().height.into(),
        })
        .collect()
}

/// Saves the current placement. It does nothing when the platform cannot tell the position
/// or when the app has no state folder, so a bad read never replaces a good file.
pub fn save(window: &WebviewWindow) -> Result<(), String> {
    let Some(dir) = position::state_dir() else {
        return Err("The app has no state folder for the window position.".into());
    };
    let Some(pos) = current(window) else {
        return Err("The app cannot read the window position.".into());
    };
    let path = position::file_in(&dir);
    position::save(&path, &pos).map_err(|e| format!("Cannot write {}. {e}", path.display()))?;
    if let Some(home) = window.try_state::<Home>() {
        home.set(pos);
    }
    Ok(())
}

/// Restores the saved placement, if the file has one that is still valid. Otherwise the
/// window keeps the default placement. On Hyprland the move waits for the first map.
pub fn restore(window: &WebviewWindow, anchor: &Anchor, home: &Home) {
    let Some(path) = position::state_dir().map(|d| position::file_in(&d)) else {
        return;
    };
    let saved = match position::load(&path) {
        Ok(Some(saved)) => saved,
        Ok(None) => return,
        Err(reason) => {
            eprintln!("usaki: {reason} The bubble uses the default placement.");
            return;
        }
    };
    let Some(at) = position::restore_point(&saved, &monitors(window)) else {
        eprintln!(
            "usaki: The saved position is not on a monitor. The bubble uses the default placement."
        );
        return;
    };
    home.set(saved.clone());
    #[cfg(target_os = "linux")]
    if on_hyprland() {
        crate::hyprland::move_after_settle(at, anchor.clone());
        return;
    }
    let _ = anchor;
    let (Ok(x), Ok(y)) = (i32::try_from(at.0), i32::try_from(at.1)) else {
        return;
    };
    if let Err(e) = window.set_position(tauri::PhysicalPosition::new(x, y)) {
        eprintln!("usaki: Cannot restore the window position. {e}");
    }
}

/// The time between two position reads while the tracker runs.
const TRACK_POLL: Duration = Duration::from_millis(200);
/// A position that holds this long counts as the end of a move, and the tracker saves it.
const TRACK_SETTLE: Duration = Duration::from_millis(400);
/// The tracker stops when the position holds this long after the last arm or move.
const TRACK_IDLE: Duration = Duration::from_secs(3);

/// Watches the window position after a drag starts and saves each position that settles.
///
/// The page cannot tell the end of a drag for sure, because the compositor holds the pointer
/// during the move. So the page arms the tracker when a drag starts and again when it sees
/// the drag end, and the backend saves what the window reports.
#[derive(Debug, Clone, Default)]
pub struct Tracker(Arc<Mutex<Option<Instant>>>);

/// What the tracker does after one read.
#[derive(Debug, PartialEq, Eq)]
pub enum TrackStep {
    Wait,
    Save,
    Stop,
}

/// Decides the next tracker step. `stable_for` is the time since the position last changed,
/// `unsaved` is true when the position differs from the last save, and `idle` is true when
/// the stop time has passed.
pub fn track_step(stable_for: Duration, unsaved: bool, idle: bool) -> TrackStep {
    if unsaved && (stable_for >= TRACK_SETTLE || idle) {
        TrackStep::Save
    } else if idle {
        TrackStep::Stop
    } else {
        TrackStep::Wait
    }
}

impl Tracker {
    /// Extends the stop time. It returns true when no tracker thread runs, so the caller
    /// must start one.
    fn arm(&self, until: Instant) -> bool {
        let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let start = slot.is_none();
        *slot = Some(slot.map_or(until, |old| old.max(until)));
        start
    }

    /// Returns true while a tracker thread runs, so a drag may still move the window.
    pub fn active(&self) -> bool {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some()
    }

    /// Returns true when the stop time has passed. The thread then ends, and the next arm
    /// starts a new one.
    fn done(&self, now: Instant) -> bool {
        let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if slot.is_some_and(|until| now >= until) {
            *slot = None;
            return true;
        }
        false
    }
}

/// Arms the tracker of `window`, and starts its thread when none runs.
pub fn track(window: &WebviewWindow, tracker: &Tracker) {
    if !tracker.arm(Instant::now() + TRACK_IDLE) {
        return;
    }
    let (window, tracker) = (window.clone(), tracker.clone());
    std::thread::spawn(move || {
        let mut last = current(&window);
        let mut changed = Instant::now();
        let mut saved: Option<SavedPosition> = None;
        loop {
            std::thread::sleep(TRACK_POLL);
            let now = Instant::now();
            let pos = current(&window);
            if pos != last {
                last = pos;
                changed = now;
                tracker.arm(now + TRACK_IDLE);
            }
            let idle = tracker.done(now);
            match track_step(now - changed, last.is_some() && last != saved, idle) {
                TrackStep::Save => {
                    if let Err(reason) = save(&window) {
                        eprintln!("usaki: {reason}");
                    }
                    saved = last.clone();
                    if idle {
                        return;
                    }
                }
                TrackStep::Stop => return,
                TrackStep::Wait => {}
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_clears_only_its_own_corner() {
        let anchor = Anchor::default();
        assert_eq!(anchor.get(), None);
        anchor.set((1, 2));
        anchor.clear_if((3, 4));
        assert_eq!(anchor.get(), Some((1, 2)));
        anchor.clear_if((1, 2));
        assert_eq!(anchor.get(), None);
    }

    #[test]
    fn tracker_saves_a_settled_position_and_stops_when_idle() {
        let short = Duration::from_millis(100);
        assert_eq!(track_step(short, true, false), TrackStep::Wait);
        assert_eq!(track_step(TRACK_SETTLE, true, false), TrackStep::Save);
        assert_eq!(track_step(TRACK_SETTLE, false, false), TrackStep::Wait);
        assert_eq!(track_step(short, true, true), TrackStep::Save);
        assert_eq!(track_step(short, false, true), TrackStep::Stop);
    }

    #[test]
    fn tracker_arms_one_thread_and_keeps_the_later_stop_time() {
        let tracker = Tracker::default();
        let now = Instant::now();
        assert!(tracker.arm(now + TRACK_IDLE));
        assert!(!tracker.arm(now + TRACK_SETTLE));
        assert!(!tracker.done(now + TRACK_SETTLE));
        assert!(tracker.done(now + TRACK_IDLE));
        assert!(!tracker.active());
        assert!(tracker.arm(now + TRACK_IDLE));
        assert!(tracker.active());
    }

    #[test]
    fn home_init_keeps_a_home_that_is_set() {
        let home = Home::default();
        home.init(SavedPosition::new(1, 2, None));
        home.init(SavedPosition::new(3, 4, None));
        assert_eq!(home.get(), Some(SavedPosition::new(1, 2, None)));
        home.set(SavedPosition::new(5, 6, None));
        assert_eq!(home.get(), Some(SavedPosition::new(5, 6, None)));
    }
}
