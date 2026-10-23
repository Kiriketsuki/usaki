//! Claude provider. It reads the cache file that the statusline writes.
//!
//! The contract is in `docs/design/claude-cache.md`.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;

use super::{Account, Clock, Provider, ProviderKind, SystemClock};
use crate::error::{Error, Result};
use crate::model::{Snapshot, Source, Window, WindowKind, kind_from_minutes};

/// A cache file older than this is stale.
pub const DEFAULT_STALE_AFTER: Duration = Duration::minutes(15);

/// A `written_at` value ahead of `now` by more than this is a bad clock or a bad writer.
pub const FUTURE_SKEW_ALLOWANCE: Duration = Duration::minutes(2);

/// Returns the base cache directory.
///
/// `xdg` is XDG_CACHE_HOME. `home` is the home directory. An empty value
/// counts as unset.
pub fn cache_dir_from(xdg: Option<&str>, home: Option<&str>) -> Result<PathBuf> {
    let non_empty = |v: Option<&str>| v.filter(|s| !s.is_empty()).map(PathBuf::from);
    if let Some(dir) = non_empty(xdg) {
        return Ok(dir);
    }
    non_empty(home)
        .map(|h| h.join(".cache"))
        .ok_or(Error::NoCacheDir)
}

/// Returns the base cache directory from the process environment.
///
/// It uses XDG_CACHE_HOME, then HOME, then USERPROFILE (Windows).
pub fn cache_dir_from_env() -> Result<PathBuf> {
    let xdg = std::env::var("XDG_CACHE_HOME").ok();
    let home = std::env::var("HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("USERPROFILE").ok());
    cache_dir_from(xdg.as_deref(), home.as_deref())
}

/// Returns true when `id` is safe to put in a file name: `[A-Za-z0-9_-]`, not empty.
pub fn is_valid_account_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Returns the cache file path for an account under `cache_dir`.
///
/// The caller must check the id with `is_valid_account_id` first.
pub fn cache_path(cache_dir: &Path, account_id: &str) -> PathBuf {
    cache_dir
        .join("usaki")
        .join(format!("claude-{account_id}.json"))
}

/// Reads the statusline cache file of a Claude account.
#[derive(Debug, Clone)]
pub struct ClaudeCacheProvider<C: Clock = SystemClock> {
    clock: C,
    stale_after: Duration,
    cache_dir: Option<PathBuf>,
}

impl ClaudeCacheProvider<SystemClock> {
    /// A provider that uses the real clock.
    pub fn system() -> Self {
        Self::new(SystemClock)
    }
}

impl<C: Clock> ClaudeCacheProvider<C> {
    pub fn new(clock: C) -> Self {
        Self {
            clock,
            stale_after: DEFAULT_STALE_AFTER,
            cache_dir: None,
        }
    }

    /// Returns a provider with another staleness threshold.
    pub fn with_stale_after(self, stale_after: Duration) -> Self {
        Self {
            stale_after,
            ..self
        }
    }

    /// Returns a provider that uses `dir` as the base cache directory.
    pub fn with_cache_dir(self, dir: PathBuf) -> Self {
        Self {
            cache_dir: Some(dir),
            ..self
        }
    }

    fn path_for(&self, acct: &Account) -> Result<PathBuf> {
        if let Some(path) = &acct.source {
            return Ok(path.clone());
        }
        let base = match &self.cache_dir {
            Some(dir) => dir.clone(),
            None => cache_dir_from_env()?,
        };
        if !is_valid_account_id(&acct.id) {
            return Err(Error::InvalidAccountId {
                account: acct.id.clone(),
            });
        }
        Ok(cache_path(&base, &acct.id))
    }
}

impl<C: Clock> Provider for ClaudeCacheProvider<C> {
    fn fetch(&self, acct: &Account) -> Result<Snapshot> {
        if acct.provider != ProviderKind::ClaudeCache {
            return Err(Error::WrongProvider {
                account: acct.id.clone(),
                expected: "claude-cache".to_string(),
            });
        }
        let path = self.path_for(acct)?;
        let now = self.clock.now();
        match std::fs::read_to_string(&path) {
            Ok(text) => parse_snapshot(&acct.id, &text, &path, now, self.stale_after),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Snapshot::new(
                &acct.id,
                Vec::new(),
                now,
                Source::ClaudeCache,
                true,
            )),
            Err(source) => Err(Error::Io { path, source }),
        }
    }
}

/// Parses the text of a cache file into a snapshot.
///
/// `origin` is the file path, used only in error messages.
pub fn parse_snapshot(
    account_id: &str,
    text: &str,
    origin: &Path,
    now: DateTime<Utc>,
    stale_after: Duration,
) -> Result<Snapshot> {
    let root: Value = serde_json::from_str(text).map_err(|e| Error::Malformed {
        path: origin.to_path_buf(),
        reason: e.to_string(),
    })?;
    let bad = |field: &str, reason: &str| Error::InvalidField {
        path: origin.to_path_buf(),
        field: field.to_string(),
        reason: reason.to_string(),
    };
    let obj = root
        .as_object()
        .ok_or_else(|| bad("(root)", "The root must be an object."))?;

    let raw_windows = match obj.get("limits") {
        Some(Value::Array(items)) => parse_limits(items, &bad)?,
        Some(Value::Null) | None => parse_flat(obj, &bad)?,
        Some(_) => return Err(bad("limits", "The value must be an array.")),
    };
    let windows = raw_windows.iter().map(|w| w.as_of(now)).collect();

    let written_at = match obj.get("written_at") {
        None | Some(Value::Null) => None,
        Some(v) => Some(parse_time(v).map_err(|r| bad("written_at", r))?),
    };
    // A `written_at` far ahead of `now` cannot be trusted, so it counts as stale.
    // A small lead is clock skew between the writer and the reader.
    let (fetched_at, stale) = match written_at {
        Some(w) if w - now > FUTURE_SKEW_ALLOWANCE => (now, true),
        Some(w) => (w.min(now), now - w > stale_after),
        None => (now, true),
    };
    Ok(Snapshot::new(
        account_id,
        windows,
        fetched_at,
        Source::ClaudeCache,
        stale,
    ))
}

type Bad<'a> = dyn Fn(&str, &str) -> Error + 'a;

fn parse_flat(obj: &serde_json::Map<String, Value>, bad: &Bad) -> Result<Vec<Window>> {
    let mut out = Vec::new();
    for (key, kind) in [
        ("five_hour", WindowKind::FiveHour),
        ("seven_day", WindowKind::SevenDay),
    ] {
        match obj.get(key) {
            None | Some(Value::Null) => {}
            Some(v) => out.push(parse_window(key, kind, v, bad)?),
        }
    }
    Ok(out)
}

fn parse_limits(items: &[Value], bad: &Bad) -> Result<Vec<Window>> {
    let mut seen: Vec<u32> = Vec::new();
    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let name = format!("limits[{i}]");
            let minutes = item
                .get("window_minutes")
                .and_then(Value::as_u64)
                .and_then(|m| u32::try_from(m).ok())
                .ok_or_else(|| {
                    bad(
                        &format!("{name}.window_minutes"),
                        "The value must be a whole number of minutes.",
                    )
                })?;
            if seen.contains(&minutes) {
                return Err(bad(
                    &format!("{name}.window_minutes"),
                    "Two entries must not have the same window_minutes.",
                ));
            }
            seen.push(minutes);
            parse_window(&name, kind_from_minutes(minutes), item, bad)
        })
        .collect()
}

fn parse_window(name: &str, kind: WindowKind, v: &Value, bad: &Bad) -> Result<Window> {
    let used = v
        .get("used_percentage")
        .and_then(Value::as_f64)
        .filter(|n| n.is_finite())
        .ok_or_else(|| {
            bad(
                &format!("{name}.used_percentage"),
                "The value must be a number.",
            )
        })?;
    // A value over 100 stays, so the UI can show an over-limit state.
    if used < 0.0 {
        return Err(bad(
            &format!("{name}.used_percentage"),
            "The value must not be negative.",
        ));
    }
    let resets_at = match v.get("resets_at") {
        None | Some(Value::Null) => None,
        Some(t) => Some(parse_time(t).map_err(|r| bad(&format!("{name}.resets_at"), r))?),
    };
    Ok(Window::new(kind, used, resets_at))
}

/// Parses epoch seconds (int or float) or an RFC 3339 string.
fn parse_time(v: &Value) -> std::result::Result<DateTime<Utc>, &'static str> {
    const REASON: &str = "The value must be epoch seconds or an RFC 3339 string.";
    match v {
        Value::Number(n) => {
            let secs = n
                .as_i64()
                .or_else(|| {
                    n.as_f64()
                        .filter(|f| f.is_finite())
                        .map(|f| f.floor() as i64)
                })
                .ok_or(REASON)?;
            DateTime::from_timestamp(secs, 0).ok_or(REASON)
        }
        Value::String(s) => DateTime::parse_from_rfc3339(s)
            .map(|d| d.with_timezone(&Utc))
            .map_err(|_| REASON),
        _ => Err(REASON),
    }
}
