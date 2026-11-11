//! Live sessions. Each file in `<cache>/<pool>.live/` is one session that wrote a heartbeat.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{HubDirs, LIVE_DIR_SUFFIX};

/// A session file is live when it is at most this old.
pub const LIVE_WINDOW_SECS: i64 = 600;

/// Returns the count of live sessions for each seat, or `None` when the folder does not exist.
///
/// A file that is not valid, or that is too old, does not count.
pub fn read_live(dirs: &HubDirs, pool: &str, now: i64) -> Option<BTreeMap<String, u32>> {
    let entries = std::fs::read_dir(dirs.cache_file(pool, LIVE_DIR_SUFFIX)).ok()?;
    let mut counts = BTreeMap::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some((seat, written_at)) = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| {
                Some((
                    v.get("seat")?.as_str()?.to_owned(),
                    v.get("written_at")?.as_i64()?,
                ))
            })
        else {
            continue;
        };
        if now - written_at <= LIVE_WINDOW_SECS {
            *counts.entry(seat).or_insert(0) += 1;
        }
    }
    Some(counts)
}
