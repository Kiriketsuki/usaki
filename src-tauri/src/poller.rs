//! The poller. It reads every account on a fixed interval and sends the result to the webview.
//!
//! `poll` holds all the logic and does not use Tauri, so tests call it with a fixed clock.
//!
//! Each Claude account has two possible sources: the statusline cache file and the watcher
//! login. `poll` merges them by account name, so an account in both sources appears once.
//! The Codex accounts come after the Claude accounts. Each group is sorted by name.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use tauri::{AppHandle, Emitter};
use usaki_core::providers::claude_cache::{cache_dir_from_env, usaki_cache_dir};
use usaki_core::providers::claude_watch::WatchRecord;
use usaki_core::{ClaudeCacheProvider, Clock, Provider, Snapshot, SystemClock};

use crate::accounts;
use crate::codex::CodexSources;
use crate::view::{AccountView, ProviderName, UsageUpdate};
use crate::watch::WatchRecords;

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

/// Reads every account once and merges the cache files with the watcher records.
pub fn poll<C: Clock + Clone>(
    cache_dir: &CacheDir,
    watched: &[WatchRecord],
    codex: &CodexSources,
    clock: C,
) -> UsageUpdate {
    let now = clock.now();
    let (mut cached, error) = read_cache(cache_dir, clock.clone());
    let ids: BTreeSet<String> = cached
        .keys()
        .cloned()
        .chain(watched.iter().map(|r| r.account_id.clone()))
        .collect();
    let accounts = ids
        .iter()
        .filter_map(|id| {
            let record = watched.iter().find(|r| &r.account_id == id);
            pick(id, cached.remove(id), record, now)
        })
        .chain(codex.read(clock, now))
        .collect();
    UsageUpdate {
        polled_at: now.timestamp_millis(),
        accounts,
        error,
    }
}

type CacheResults = BTreeMap<String, usaki_core::Result<Snapshot>>;

/// Reads every cache file. The second value is the problem that stopped the discovery.
fn read_cache<C: Clock>(cache_dir: &CacheDir, clock: C) -> (CacheResults, Option<String>) {
    let dir = match &cache_dir.0 {
        Ok(dir) => dir,
        Err(reason) => return (BTreeMap::new(), Some(reason.clone())),
    };
    let accts = match accounts::discover(dir) {
        Ok(accts) => accts,
        Err(e) => return (BTreeMap::new(), Some(folder_error(dir, &e))),
    };
    let provider = ClaudeCacheProvider::new(clock).with_cache_dir(dir.clone());
    let results = accts
        .iter()
        .map(|a| (a.id.clone(), provider.fetch(a)))
        .collect();
    (results, None)
}

/// Picks the source of one account.
///
/// 1. A fresh cache file wins.
/// 2. Else a watcher record that has a reading.
/// 3. Else the cache result, stale or failed.
/// 4. Else the watcher record without a reading, which carries the reason.
fn pick(
    id: &str,
    cached: Option<usaki_core::Result<Snapshot>>,
    record: Option<&WatchRecord>,
    now: DateTime<Utc>,
) -> Option<AccountView> {
    let view = |result| AccountView::from_result(ProviderName::Claude, id, result, now);
    match (cached, record) {
        (Some(Ok(snap)), _) if !snap.stale => Some(view(Ok(snap))),
        (Some(result), Some(rec)) if rec.reading.is_none() => Some(view(result)),
        (_, Some(rec)) => Some(view(Ok(rec.snapshot(now)))),
        (cached, None) => cached.map(view),
    }
}

fn folder_error(dir: &Path, e: &std::io::Error) -> String {
    format!(
        "Cannot read the cache folder {}. {e}",
        usaki_cache_dir(dir).display()
    )
}

/// Runs `poll` on a blocking thread, because it reads files.
pub async fn poll_in_background(
    cache_dir: CacheDir,
    watched: WatchRecords,
    codex: CodexSources,
) -> Result<UsageUpdate, String> {
    tauri::async_runtime::spawn_blocking(move || {
        poll(&cache_dir, &watched.get(), &codex, SystemClock)
    })
    .await
    .map_err(|e| format!("The poll task failed. {e}"))
}

/// Starts the poll loop. The first tick runs at once.
pub fn spawn(app: AppHandle, cache_dir: CacheDir, watched: WatchRecords, codex: CodexSources) {
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(POLL_INTERVAL);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            match poll_in_background(cache_dir.clone(), watched.clone(), codex.clone()).await {
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
    use crate::codex::CodexHome;
    use chrono::TimeZone;
    use usaki_core::providers::claude_watch::{Problem, Reading};
    use usaki_core::{Window, WindowKind};

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

    fn none() -> CodexSources {
        CodexSources::default()
    }

    /// Writes one synthetic Codex session log under `home`, with a 7d window only.
    fn codex_home(home: &Path, pct: u32) {
        let day = home.join("sessions/2027/01/15");
        std::fs::create_dir_all(&day).unwrap();
        let line = format!(
            r#"{{"timestamp":"2027-01-15T08:00:00Z","type":"event_msg","payload":{{"type":"token_count","rate_limits":{{"limit_id":"codex","primary":{{"used_percent":{pct},"window_minutes":10080,"resets_at":1800300000}},"secondary":null,"credits":{{"has_credits":true,"unlimited":false,"balance":"5.00"}}}}}}}}"#
        );
        std::fs::write(day.join("rollout-a.jsonl"), line + "\n").unwrap();
    }

    fn codex(homes: &[(&str, &Path)]) -> CodexSources {
        CodexSources::new(
            homes
                .iter()
                .map(|(name, home)| CodexHome {
                    name: (*name).to_owned(),
                    home: home.to_path_buf(),
                })
                .collect(),
        )
    }

    fn ids(update: &UsageUpdate) -> Vec<&str> {
        update.accounts.iter().map(|a| a.id.as_str()).collect()
    }

    #[test]
    fn codex_accounts_follow_the_claude_accounts_by_name() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "main", &flat(42, NOW - 60));
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        codex_home(a.path(), 37);
        codex_home(b.path(), 5);
        let sources = codex(&[("zeta", a.path()), ("main", b.path())]);
        let update = poll(&cache(dir.path()), &[], &sources, clock());
        assert_eq!(ids(&update), ["claude:main", "codex:main", "codex:zeta"]);
        let zeta = &update.accounts[2];
        assert!(!zeta.stale);
        assert_eq!(zeta.provider, ProviderName::Codex);
        assert_eq!(zeta.name, "zeta");
        assert_eq!(zeta.windows.len(), 1);
        assert_eq!(zeta.windows[0].minutes, 10080);
        assert_eq!(zeta.windows[0].used_pct, 37.0);
        assert_eq!(
            zeta.credits.as_ref().and_then(|c| c.balance.as_deref()),
            Some("5.00")
        );
    }

    #[test]
    fn codex_home_without_sessions_gives_no_account() {
        let dir = tempfile::tempdir().unwrap();
        let empty = tempfile::tempdir().unwrap();
        let update = poll(
            &cache(dir.path()),
            &[],
            &codex(&[("codex", empty.path())]),
            clock(),
        );
        assert!(update.accounts.is_empty());
    }

    #[test]
    fn reads_every_account_in_order() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "acc2", &flat(81, NOW - 60));
        write(dir.path(), "acc1", &flat(42, NOW - 60));
        let update = poll(&cache(dir.path()), &[], &none(), clock());
        assert_eq!(update.error, None);
        assert_eq!(update.polled_at, NOW * 1000);
        assert_eq!(ids(&update), ["claude:acc1", "claude:acc2"]);
        assert!(update.accounts.iter().all(|a| !a.stale));
        assert_eq!(update.accounts[0].windows[0].used_pct, 42.0);
        assert_eq!(update.accounts[1].windows[0].used_pct, 81.0);
    }

    #[test]
    fn old_file_is_stale() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "acc1", &flat(42, NOW - 3600));
        let update = poll(&cache(dir.path()), &[], &none(), clock());
        assert!(update.accounts[0].stale);
    }

    #[test]
    fn bad_file_gives_a_stale_account_with_an_error() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "acc1", "{ not json");
        write(dir.path(), "acc2", &flat(10, NOW));
        let update = poll(&cache(dir.path()), &[], &none(), clock());
        assert!(update.accounts[0].stale);
        assert!(update.accounts[0].error.is_some());
        assert!(!update.accounts[1].stale);
    }

    #[test]
    fn empty_cache_gives_no_accounts() {
        let dir = tempfile::tempdir().unwrap();
        let update = poll(&cache(dir.path()), &[], &none(), clock());
        assert!(update.accounts.is_empty());
        assert_eq!(update.error, None);
    }

    fn watched(id: &str, pct: f64, fetched_at: i64) -> WatchRecord {
        WatchRecord {
            account_id: id.to_owned(),
            reading: Some(Reading {
                windows: vec![Window::new(WindowKind::FiveHour, pct, None)],
                fetched_at: Utc.timestamp_opt(fetched_at, 0).unwrap(),
            }),
            problem: None,
        }
    }

    fn unread(id: &str) -> WatchRecord {
        WatchRecord {
            account_id: id.to_owned(),
            reading: None,
            problem: Some(Problem::NoCredentials),
        }
    }

    fn pct(update: &UsageUpdate, i: usize) -> f64 {
        update.accounts[i].windows[0].used_pct
    }

    #[test]
    fn watcher_only_account_appears() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "acc1", &flat(42, NOW - 60));
        let update = poll(
            &cache(dir.path()),
            &[watched("acc2", 7.0, NOW - 60)],
            &none(),
            clock(),
        );
        assert_eq!(ids(&update), ["claude:acc1", "claude:acc2"]);
        assert!(!update.accounts[1].stale);
        assert_eq!(pct(&update, 1), 7.0);
    }

    #[test]
    fn fresh_cache_wins_and_the_account_appears_once() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "acc1", &flat(42, NOW - 60));
        let update = poll(
            &cache(dir.path()),
            &[watched("acc1", 7.0, NOW - 60)],
            &none(),
            clock(),
        );
        assert_eq!(update.accounts.len(), 1);
        assert_eq!(pct(&update, 0), 42.0);
    }

    #[test]
    fn stale_cache_gives_way_to_the_watcher() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "acc1", &flat(42, NOW - 3600));
        let update = poll(
            &cache(dir.path()),
            &[watched("acc1", 7.0, NOW - 60)],
            &none(),
            clock(),
        );
        assert_eq!(update.accounts.len(), 1);
        assert!(!update.accounts[0].stale);
        assert_eq!(pct(&update, 0), 7.0);
    }

    #[test]
    fn stale_cache_stays_when_the_watcher_has_no_reading() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "acc1", &flat(42, NOW - 3600));
        let update = poll(&cache(dir.path()), &[unread("acc1")], &none(), clock());
        assert!(update.accounts[0].stale);
        assert_eq!(pct(&update, 0), 42.0);
    }

    #[test]
    fn watcher_without_reading_shows_its_reason() {
        let dir = tempfile::tempdir().unwrap();
        let update = poll(&cache(dir.path()), &[unread("acc1")], &none(), clock());
        assert!(update.accounts[0].stale);
        assert!(update.accounts[0].windows.is_empty());
        assert!(
            update.accounts[0]
                .error
                .as_deref()
                .unwrap()
                .contains("credentials")
        );
    }

    #[test]
    fn watcher_accounts_show_without_a_cache_dir() {
        let update = poll(
            &CacheDir(Err("no dir".into())),
            &[watched("acc1", 7.0, NOW - 60)],
            &none(),
            clock(),
        );
        assert_eq!(update.error.as_deref(), Some("no dir"));
        assert_eq!(update.accounts.len(), 1);
        assert_eq!(pct(&update, 0), 7.0);
    }

    #[test]
    fn unknown_cache_dir_gives_an_error() {
        let update = poll(&CacheDir(Err("no dir".into())), &[], &none(), clock());
        assert!(update.accounts.is_empty());
        assert_eq!(update.error.as_deref(), Some("no dir"));
    }
}
