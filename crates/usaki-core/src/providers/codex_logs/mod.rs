//! Codex provider. It reads the rate limits from the Codex session logs.
//!
//! Codex writes a `token_count` event to the session log of each running session. The event
//! holds `rate_limits` with up to two windows, `primary` and `secondary`. The provider maps
//! each window by `window_minutes`, because a plan can put the 7d window in either slot. On
//! the Pro plan Codex reports only the 7d window.
//!
//! The provider never reads `auth.json`, never calls an OpenAI endpoint and never starts
//! `codex app-server`. It never logs a line of a session log, because the logs hold whole
//! conversations.
//!
//! The design is in `docs/design/codex-logs.md`.

pub mod event;
pub mod scan;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use super::{Account, Clock, Provider, ProviderKind, SystemClock};
use crate::error::{Error, Result};
use crate::model::{Snapshot, Source};

pub use event::{CODEX_LIMIT_ID, CodexEvent, parse_line};
pub use scan::{Latest, ScanCache, last_event, newest_event};

/// The environment variable that moves the Codex home.
pub const CODEX_HOME_ENV: &str = "CODEX_HOME";
/// The folder under the Codex home that holds the session logs.
pub const SESSIONS_DIR: &str = "sessions";

/// The logs update only while Codex runs. An event older than this makes the account stale,
/// unless one of its windows is still open.
pub const STALE_AFTER: Duration = Duration::hours(6);

/// Returns the Codex home.
///
/// `codex_home` is CODEX_HOME. `home` is the home directory. An empty value counts as
/// unset. The default is `<home>/.codex`.
pub fn codex_home_from(codex_home: Option<&str>, home: Option<&str>) -> Result<PathBuf> {
    let non_empty = |v: Option<&str>| v.filter(|s| !s.is_empty()).map(PathBuf::from);
    if let Some(dir) = non_empty(codex_home) {
        return Ok(dir);
    }
    non_empty(home)
        .map(|h| h.join(".codex"))
        .ok_or(Error::NoCodexHome)
}

/// Returns the Codex home from the process environment.
///
/// It uses CODEX_HOME, then HOME, then USERPROFILE (Windows).
pub fn codex_home_from_env() -> Result<PathBuf> {
    let codex_home = std::env::var(CODEX_HOME_ENV).ok();
    let home = std::env::var("HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("USERPROFILE").ok());
    codex_home_from(codex_home.as_deref(), home.as_deref())
}

/// Returns the sessions folder of a Codex home.
pub fn sessions_dir(codex_home: &Path) -> PathBuf {
    codex_home.join(SESSIONS_DIR)
}

/// Reads the newest rate limit event of one Codex home.
///
/// The account source is the Codex home. Without a source, the provider uses the Codex home
/// from the environment.
#[derive(Debug, Clone)]
pub struct CodexLogsProvider<C: Clock = SystemClock> {
    clock: C,
    stale_after: Duration,
    cache: Arc<ScanCache>,
}

impl CodexLogsProvider<SystemClock> {
    /// A provider that uses the real clock.
    pub fn system() -> Self {
        Self::new(SystemClock)
    }
}

impl<C: Clock> CodexLogsProvider<C> {
    pub fn new(clock: C) -> Self {
        Self {
            clock,
            stale_after: STALE_AFTER,
            cache: Arc::default(),
        }
    }

    /// Returns a provider with another staleness threshold.
    pub fn with_stale_after(self, stale_after: Duration) -> Self {
        Self {
            stale_after,
            ..self
        }
    }

    /// Returns a provider that shares `cache`, so a new provider does not read the same
    /// files again.
    pub fn with_cache(self, cache: Arc<ScanCache>) -> Self {
        Self { cache, ..self }
    }
}

impl<C: Clock> Provider for CodexLogsProvider<C> {
    fn fetch(&self, acct: &Account) -> Result<Snapshot> {
        if acct.provider != ProviderKind::CodexLogs {
            return Err(Error::WrongProvider {
                account: acct.id.clone(),
                expected: "codex-logs".to_string(),
            });
        }
        let home = match &acct.source {
            Some(path) => path.clone(),
            None => codex_home_from_env()?,
        };
        let sessions = sessions_dir(&home);
        let now = self.clock.now();
        let empty = |reason: &str| {
            Snapshot::new(&acct.id, Vec::new(), now, Source::CodexLogs, true)
                .with_stale_reason(reason)
        };
        match newest_event(&sessions, &self.cache) {
            Ok(Some(latest)) => Ok(snapshot(&acct.id, &latest, now, self.stale_after)),
            Ok(None) => Ok(empty("The Codex session logs hold no rate limit event.")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(empty("The Codex home has no session logs."))
            }
            Err(source) => Err(Error::Io {
                path: sessions,
                source,
            }),
        }
    }
}

/// Returns the snapshot of the newest event at `now`.
///
/// A window whose reset time has passed reports 0%. The snapshot is stale when the event is
/// older than `stale_after` and no window is still open. An open window keeps its percent
/// as a lower bound, because a usage after the event cannot lower it.
pub fn snapshot(
    account_id: &str,
    latest: &Latest,
    now: DateTime<Utc>,
    stale_after: Duration,
) -> Snapshot {
    let open = latest
        .event
        .windows
        .iter()
        .any(|w| w.resets_at.is_some_and(|r| r > now));
    let stale = now - latest.at > stale_after && !open;
    let windows = latest.event.windows.iter().map(|w| w.as_of(now)).collect();
    let snap = Snapshot::new(
        account_id,
        windows,
        latest.at.min(now),
        Source::CodexLogs,
        stale,
    )
    .with_credits(latest.event.credits.clone());
    if stale {
        snap.with_stale_reason(format!(
            "The newest Codex event is older than {} hours, and no window is still open.",
            stale_after.num_hours()
        ))
    } else {
        snap
    }
}
