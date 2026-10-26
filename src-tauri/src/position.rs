//! The saved window position.
//!
//! The app saves the top-left corner of the window when a drag ends and on exit, and it
//! restores the corner on start. The file is small JSON with a format version:
//!
//! ```json
//! {"version":1,"x":1494,"y":490,"monitor":"DP-2"}
//! ```
//!
//! On Hyprland the point is in the global layout coordinates of `hyprctl`. Elsewhere it is in
//! the physical pixels of Tauri. Each machine keeps its own file, so the two never mix.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The format version that this build reads and writes.
pub const FORMAT_VERSION: u32 = 1;
/// The file name inside the state folder.
pub const FILE_NAME: &str = "position.json";
/// An environment variable that replaces the state folder, for tests and live checks.
pub const STATE_DIR_ENV: &str = "USAKI_STATE_DIR";

/// The saved top-left corner and the monitor that held it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedPosition {
    pub version: u32,
    pub x: i64,
    pub y: i64,
    /// The monitor name, when the platform gives one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitor: Option<String>,
}

impl SavedPosition {
    /// Returns a position in the current format.
    pub fn new(x: i64, y: i64, monitor: Option<String>) -> Self {
        Self {
            version: FORMAT_VERSION,
            x,
            y,
            monitor,
        }
    }
}

/// One monitor in the same coordinates as the saved point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorRect {
    pub name: Option<String>,
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

impl MonitorRect {
    /// Returns true when (x, y) lies on this monitor.
    pub fn contains(&self, x: i64, y: i64) -> bool {
        (self.x..self.x + self.width).contains(&x) && (self.y..self.y + self.height).contains(&y)
    }
}

/// Returns the state folder: `USAKI_STATE_DIR`, else `dirs::state_dir()/usaki`, else
/// `dirs::data_dir()/usaki`. Windows and macOS have no state folder, so they use the data
/// folder.
pub fn state_dir() -> Option<PathBuf> {
    state_dir_from(
        std::env::var_os(STATE_DIR_ENV),
        dirs::state_dir().or_else(dirs::data_dir),
    )
}

/// Returns the state folder from the override and the platform folder. An empty override
/// counts as no override.
pub fn state_dir_from(over: Option<OsString>, platform: Option<PathBuf>) -> Option<PathBuf> {
    over.filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| platform.map(|d| d.join("usaki")))
}

/// Returns the path of the position file in `dir`.
pub fn file_in(dir: &Path) -> PathBuf {
    dir.join(FILE_NAME)
}

/// Reads the position file. A missing file gives `Ok(None)`. A file that is not valid JSON or
/// that has another format version gives an error, and the caller uses the default placement.
pub fn load(path: &Path) -> Result<Option<SavedPosition>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Cannot read {}. {e}", path.display())),
    };
    let pos: SavedPosition = serde_json::from_str(&text)
        .map_err(|e| format!("The file {} is not a valid position. {e}", path.display()))?;
    if pos.version != FORMAT_VERSION {
        return Err(format!(
            "The file {} has format version {}. This build reads version {FORMAT_VERSION}.",
            path.display(),
            pos.version
        ));
    }
    Ok(Some(pos))
}

/// Writes the position file through a temporary file and a rename, so a crash never leaves
/// half a file.
pub fn save(path: &Path, pos: &SavedPosition) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(pos).map_err(io::Error::other)?)?;
    std::fs::rename(&tmp, path)
}

/// Returns the point to restore, or None for the default placement. The point is not valid
/// when the saved monitor no longer exists, or when the point lies off every monitor.
pub fn restore_point(saved: &SavedPosition, monitors: &[MonitorRect]) -> Option<(i64, i64)> {
    if let Some(name) = &saved.monitor
        && !monitors.iter().any(|m| m.name.as_deref() == Some(name))
    {
        return None;
    }
    monitors
        .iter()
        .any(|m| m.contains(saved.x, saved.y))
        .then_some((saved.x, saved.y))
}

/// Returns the name of the monitor that holds (x, y), if one does.
pub fn monitor_at(monitors: &[MonitorRect], x: i64, y: i64) -> Option<String> {
    monitors
        .iter()
        .find(|m| m.contains(x, y))
        .and_then(|m| m.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(name: &str, x: i64, width: i64) -> MonitorRect {
        MonitorRect {
            name: Some(name.to_owned()),
            x,
            y: 0,
            width,
            height: 1080,
        }
    }

    fn monitors() -> Vec<MonitorRect> {
        vec![
            monitor("eDP-1", 0, 1920),
            monitor("DP-2", 1920, 1920),
            monitor("DP-3", 3840, 2560),
        ]
    }

    #[test]
    fn round_trips_through_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = file_in(&dir.path().join("usaki"));
        let pos = SavedPosition::new(1494, 490, Some("DP-2".into()));
        save(&path, &pos).unwrap();
        assert_eq!(load(&path).unwrap(), Some(pos));
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn writes_a_small_versioned_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = file_in(dir.path());
        save(&path, &SavedPosition::new(10, -20, Some("DP-2".into()))).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            r#"{"version":1,"x":10,"y":-20,"monitor":"DP-2"}"#
        );
        save(&path, &SavedPosition::new(10, 20, None)).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            r#"{"version":1,"x":10,"y":20}"#
        );
        assert_eq!(load(&path).unwrap(), Some(SavedPosition::new(10, 20, None)));
    }

    #[test]
    fn missing_file_gives_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(&file_in(dir.path())).unwrap(), None);
    }

    #[test]
    fn bad_json_or_another_version_gives_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = file_in(dir.path());
        std::fs::write(&path, "{ not json").unwrap();
        assert!(load(&path).unwrap_err().contains("not a valid position"));
        std::fs::write(&path, r#"{"version":2,"x":1,"y":2}"#).unwrap();
        assert!(load(&path).unwrap_err().contains("format version 2"));
    }

    #[test]
    fn restores_a_point_on_its_monitor() {
        let saved = SavedPosition::new(2804, 463, Some("DP-2".into()));
        assert_eq!(restore_point(&saved, &monitors()), Some((2804, 463)));
        let unnamed = SavedPosition::new(5000, 10, None);
        assert_eq!(restore_point(&unnamed, &monitors()), Some((5000, 10)));
    }

    #[test]
    fn falls_back_when_the_saved_monitor_is_gone() {
        let saved = SavedPosition::new(2804, 463, Some("HDMI-A-1".into()));
        assert_eq!(restore_point(&saved, &monitors()), None);
        assert_eq!(
            restore_point(&SavedPosition::new(10, 10, Some("DP-2".into())), &[]),
            None
        );
    }

    #[test]
    fn falls_back_when_the_point_is_off_every_monitor() {
        let below = SavedPosition::new(2000, 1080, Some("DP-2".into()));
        assert_eq!(restore_point(&below, &monitors()), None);
        let right = SavedPosition::new(6400, 10, None);
        assert_eq!(restore_point(&right, &monitors()), None);
        let left = SavedPosition::new(-1, 10, None);
        assert_eq!(restore_point(&left, &monitors()), None);
    }

    #[test]
    fn names_the_monitor_under_a_point() {
        assert_eq!(monitor_at(&monitors(), 1920, 0).as_deref(), Some("DP-2"));
        assert_eq!(
            monitor_at(&monitors(), 1919, 1079).as_deref(),
            Some("eDP-1")
        );
        assert_eq!(monitor_at(&monitors(), 9000, 0), None);
    }

    #[test]
    fn override_replaces_the_state_folder() {
        let platform = Some(PathBuf::from("/home/u/.local/state"));
        assert_eq!(
            state_dir_from(Some("/tmp/s".into()), platform.clone()),
            Some(PathBuf::from("/tmp/s"))
        );
        assert_eq!(
            state_dir_from(Some("".into()), platform.clone()),
            Some(PathBuf::from("/home/u/.local/state/usaki"))
        );
        assert_eq!(
            state_dir_from(None, platform),
            Some(PathBuf::from("/home/u/.local/state/usaki"))
        );
        assert_eq!(state_dir_from(None, None), None);
    }
}
