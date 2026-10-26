//! The poller. It reads every account on a fixed interval and sends the result to the webview.
//!
//! `poll` holds all the logic and does not use Tauri, so tests call it with a fixed clock.

use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use usaki_core::providers::claude_cache::{cache_dir_from_env, usaki_cache_dir};
use usaki_core::{ClaudeCacheProvider, Clock, Provider, SystemClock};

use crate::accounts;
use crate::view::{AccountView, UsageUpdate};

/// The time between two polls.
pub const POLL_INTERVAL: Duration = Duration::from_secs(30);

/// The event that carries each `UsageUpdate` to the webview.
pub const UPDATE_EVENT: &str = "usage://update";

/// The base cache directory, or the reason that the app could not find it.
///
/// The app resolves it once at start. A `String` error keeps the state cheap to clone.
#[derive(Debug, Clone)]
pub struct CacheDir(pub Result<PathBuf, String>);

impl CacheDir {
    /// Resolves the base cache directory from the process environment.
    pub fn from_env() -> Self {
        Self(cache_dir_from_env().map_err(|e| e.to_string()))
    }
}

/// Reads every account under the cache directory once.
pub fn poll<C: Clock + Clone>(cache_dir: &CacheDir, clock: C) -> UsageUpdate {
    let now = clock.now();
    let failed = |reason: String| UsageUpdate {
        polled_at: now.timestamp_millis(),
        accounts: Vec::new(),
        error: Some(reason),
    };
    let dir = match &cache_dir.0 {
        Ok(dir) => dir,
        Err(reason) => return failed(reason.clone()),
    };
    let accts = match accounts::discover(dir) {
        Ok(accts) => accts,
        Err(e) => return failed(folder_error(dir, &e)),
    };
    let provider = ClaudeCacheProvider::new(clock).with_cache_dir(dir.clone());
    UsageUpdate {
        polled_at: now.timestamp_millis(),
        accounts: accts
            .iter()
            .map(|a| AccountView::from_result(&a.id, provider.fetch(a), now))
            .collect(),
        error: None,
    }
}

fn folder_error(dir: &Path, e: &std::io::Error) -> String {
    format!(
        "Cannot read the cache folder {}. {e}",
        usaki_cache_dir(dir).display()
    )
}

/// Runs `poll` on a blocking thread, because it reads files.
pub async fn poll_in_background(cache_dir: CacheDir) -> Result<UsageUpdate, String> {
    tauri::async_runtime::spawn_blocking(move || poll(&cache_dir, SystemClock))
        .await
        .map_err(|e| format!("The poll task failed. {e}"))
}

/// Starts the poll loop. The first tick runs at once.
pub fn spawn(app: AppHandle, cache_dir: CacheDir) {
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(POLL_INTERVAL);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            match poll_in_background(cache_dir.clone()).await {
                Ok(update) => {
                    if let Err(e) = app.emit(UPDATE_EVENT, &update) {
                        eprintln!("usaki: Cannot send the usage update. {e}");
                    }
                }
                Err(e) => eprintln!("usaki: {e}"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, TimeZone, Utc};

    const NOW: i64 = 1_800_000_060;

    #[derive(Clone, Copy)]
    struct FixedClock(DateTime<Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    fn clock() -> FixedClock {
        FixedClock(Utc.timestamp_opt(NOW, 0).unwrap())
    }

    fn write(dir: &Path, id: &str, body: &str) {
        let usaki = usaki_cache_dir(dir);
        std::fs::create_dir_all(&usaki).unwrap();
        std::fs::write(usaki.join(format!("claude-{id}.json")), body).unwrap();
    }

    fn flat(pct: u32, written_at: i64) -> String {
        format!(
            r#"{{"five_hour":{{"used_percentage":{pct},"resets_at":1800003600}},
                "seven_day":{{"used_percentage":10,"resets_at":1800300000}},
                "written_at":{written_at}}}"#
        )
    }

    fn cache(dir: &Path) -> CacheDir {
        CacheDir(Ok(dir.to_path_buf()))
    }

    #[test]
    fn reads_every_account_in_order() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "acc2", &flat(81, NOW - 60));
        write(dir.path(), "acc1", &flat(42, NOW - 60));
        let update = poll(&cache(dir.path()), clock());
        assert_eq!(update.error, None);
        assert_eq!(update.polled_at, NOW * 1000);
        let ids: Vec<&str> = update.accounts.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["acc1", "acc2"]);
        assert!(update.accounts.iter().all(|a| !a.stale));
        assert_eq!(update.accounts[0].windows[0].used_pct, 42.0);
        assert_eq!(update.accounts[1].windows[0].used_pct, 81.0);
    }

    #[test]
    fn old_file_is_stale() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "acc1", &flat(42, NOW - 3600));
        let update = poll(&cache(dir.path()), clock());
        assert!(update.accounts[0].stale);
    }

    #[test]
    fn bad_file_gives_a_stale_account_with_an_error() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "acc1", "{ not json");
        write(dir.path(), "acc2", &flat(10, NOW));
        let update = poll(&cache(dir.path()), clock());
        assert!(update.accounts[0].stale);
        assert!(update.accounts[0].error.is_some());
        assert!(!update.accounts[1].stale);
    }

    #[test]
    fn empty_cache_gives_no_accounts() {
        let dir = tempfile::tempdir().unwrap();
        let update = poll(&cache(dir.path()), clock());
        assert!(update.accounts.is_empty());
        assert_eq!(update.error, None);
    }

    #[test]
    fn unknown_cache_dir_gives_an_error() {
        let update = poll(&CacheDir(Err("no dir".into())), clock());
        assert!(update.accounts.is_empty());
        assert_eq!(update.error.as_deref(), Some("no dir"));
    }
}
