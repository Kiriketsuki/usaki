//! Usage model shared by all providers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Length of the 5h window in minutes.
pub const FIVE_HOUR_MINUTES: u32 = 300;
/// Length of the 7d window in minutes.
pub const SEVEN_DAY_MINUTES: u32 = 10080;

/// The kind of a usage window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WindowKind {
    FiveHour,
    SevenDay,
    /// A window with another length, in minutes.
    Other(u32),
}

/// Maps a window length in minutes to its kind.
///
/// The Claude and Codex providers both use this function. Codex names its
/// windows by position, so the length is the only reliable key.
pub fn kind_from_minutes(minutes: u32) -> WindowKind {
    match minutes {
        FIVE_HOUR_MINUTES => WindowKind::FiveHour,
        SEVEN_DAY_MINUTES => WindowKind::SevenDay,
        other => WindowKind::Other(other),
    }
}

/// One usage window of an account.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Window {
    pub kind: WindowKind,
    pub used_pct: f64,
    pub resets_at: Option<DateTime<Utc>>,
}

impl Window {
    pub fn new(kind: WindowKind, used_pct: f64, resets_at: Option<DateTime<Utc>>) -> Self {
        Self {
            kind,
            used_pct,
            resets_at,
        }
    }

    /// Returns this window as it stands at `now`.
    ///
    /// A window whose reset time has passed has reset, so it reports 0% used.
    /// The reset time stays as it is.
    pub fn as_of(&self, now: DateTime<Utc>) -> Self {
        match self.resets_at {
            Some(reset) if reset <= now => Self {
                used_pct: 0.0,
                ..self.clone()
            },
            _ => self.clone(),
        }
    }
}

/// Where a snapshot came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    ClaudeCache,
    /// The usage API, called with the token of a watcher login.
    ClaudeWatch,
}

/// The usage of one account at one moment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub account_id: String,
    pub windows: Vec<Window>,
    pub fetched_at: DateTime<Utc>,
    pub source: Source,
    pub stale: bool,
    /// Why the snapshot is stale, when the provider knows the reason.
    #[serde(default)]
    pub stale_reason: Option<String>,
}

impl Snapshot {
    pub fn new(
        account_id: impl Into<String>,
        windows: Vec<Window>,
        fetched_at: DateTime<Utc>,
        source: Source,
        stale: bool,
    ) -> Self {
        Self {
            account_id: account_id.into(),
            windows,
            fetched_at,
            source,
            stale,
            stale_reason: None,
        }
    }

    /// Returns this snapshot with a stale reason.
    pub fn with_stale_reason(self, reason: impl Into<String>) -> Self {
        Self {
            stale_reason: Some(reason.into()),
            ..self
        }
    }

    /// Returns the window of the given kind, if the snapshot has one.
    pub fn window(&self, kind: WindowKind) -> Option<&Window> {
        self.windows.iter().find(|w| w.kind == kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(secs, 0).unwrap()
    }

    #[test]
    fn maps_300_to_five_hour() {
        assert_eq!(kind_from_minutes(300), WindowKind::FiveHour);
    }

    #[test]
    fn maps_10080_to_seven_day() {
        assert_eq!(kind_from_minutes(10080), WindowKind::SevenDay);
    }

    #[test]
    fn maps_other_lengths_to_other() {
        assert_eq!(kind_from_minutes(60), WindowKind::Other(60));
        assert_eq!(kind_from_minutes(0), WindowKind::Other(0));
    }

    #[test]
    fn past_reset_reports_zero_and_keeps_reset_time() {
        let w = Window::new(WindowKind::FiveHour, 80.0, Some(at(100)));
        let out = w.as_of(at(200));
        assert_eq!(out.used_pct, 0.0);
        assert_eq!(out.resets_at, Some(at(100)));
        assert_eq!(w.used_pct, 80.0, "the input stays unchanged");
    }

    #[test]
    fn future_reset_keeps_percent() {
        let w = Window::new(WindowKind::FiveHour, 80.0, Some(at(300)));
        assert_eq!(w.as_of(at(200)), w);
    }

    #[test]
    fn no_reset_time_keeps_percent() {
        let w = Window::new(WindowKind::SevenDay, 10.0, None);
        assert_eq!(w.as_of(at(200)), w);
    }
}
