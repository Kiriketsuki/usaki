//! The watcher loop. It polls the Claude watcher logins on a thread of its own.
//!
//! A usage call or a token refresh can take minutes, so the loop does not share the 30 s poll
//! tick. The watcher in usaki-core keeps the 5-minute call schedule of each account. The loop
//! skips an account whose statusline cache is fresh, because the poller then uses the cache.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use usaki_core::providers::claude_watch::{
    ClaudeBinary, UreqClient, WatchRecord, Watcher, watch_root_from_env,
};
use usaki_core::{Account, ClaudeCacheProvider, Clock, Provider, ProviderKind, SystemClock};

use crate::poller::{self, CacheDir, UPDATE_EVENT};

/// The time between two watcher ticks. The call schedule of each account is longer.
pub const WATCH_TICK: Duration = Duration::from_secs(30);

/// The latest watcher records, shared between the watcher thread and the poller.
#[derive(Debug, Clone, Default)]
pub struct WatchRecords(Arc<Mutex<Vec<WatchRecord>>>);

impl WatchRecords {
    pub fn get(&self) -> Vec<WatchRecord> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn set(&self, records: Vec<WatchRecord>) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = records;
    }
}

/// Returns true when the statusline cache of `id` is fresh.
pub fn cache_is_fresh<C: Clock>(cache_dir: &CacheDir, id: &str, clock: C) -> bool {
    let Ok(dir) = &cache_dir.0 else {
        return false;
    };
    let acct = Account::new(id, ProviderKind::ClaudeCache, None);
    ClaudeCacheProvider::new(clock)
        .with_cache_dir(dir.clone())
        .fetch(&acct)
        .is_ok_and(|snap| !snap.stale)
}

/// Starts the watcher thread. Without a data directory the app runs without watchers.
pub fn spawn(app: AppHandle, cache_dir: CacheDir, records: WatchRecords) {
    let root = match watch_root_from_env() {
        Ok(root) => root,
        Err(e) => {
            eprintln!("usaki: {e} The app does not read watcher logins.");
            return;
        }
    };
    let started = std::thread::Builder::new()
        .name("usaki-watch".into())
        .spawn(move || {
            let cli = ClaudeBinary::find();
            if cli.is_none() {
                eprintln!("usaki: Cannot find the claude binary. Watcher tokens cannot refresh.");
            }
            let mut watcher = Watcher::new(root, SystemClock, UreqClient::new(), cli);
            loop {
                let fresh = |id: &str| cache_is_fresh(&cache_dir, id, SystemClock);
                match watcher.tick(fresh) {
                    Ok(latest) => publish(&app, &cache_dir, &records, latest),
                    Err(e) => eprintln!(
                        "usaki: Cannot read the watch folder {}. {e}",
                        watcher.root().display()
                    ),
                }
                std::thread::sleep(WATCH_TICK);
            }
        });
    if let Err(e) = started {
        eprintln!("usaki: Cannot start the watcher thread. {e}");
    }
}

/// Stores new records. When they changed, it logs each change and sends an update at once.
fn publish(
    app: &AppHandle,
    cache_dir: &CacheDir,
    records: &WatchRecords,
    latest: Vec<WatchRecord>,
) {
    let before = records.get();
    if latest == before {
        return;
    }
    let now = SystemClock.now();
    for rec in latest.iter().filter(|r| !before.contains(r)) {
        eprintln!("usaki: watch {}: {}", rec.account_id, describe(rec, now));
    }
    records.set(latest);
    let update = poller::poll(cache_dir, &records.get(), SystemClock);
    if let Err(e) = app.emit(UPDATE_EVENT, &update) {
        eprintln!("usaki: Cannot send the usage update. {e}");
    }
}

/// Returns one log line for a record. It holds percents and reasons, never a token.
fn describe(rec: &WatchRecord, now: chrono::DateTime<chrono::Utc>) -> String {
    let snap = rec.snapshot(now);
    let windows: Vec<String> = snap
        .windows
        .iter()
        .map(|w| format!("{:?} {:.0}%", w.kind, w.used_pct))
        .collect();
    match snap.stale_reason {
        Some(reason) => format!("stale. {reason} [{}]", windows.join(", ")),
        None => format!("fresh [{}]", windows.join(", ")),
    }
}
