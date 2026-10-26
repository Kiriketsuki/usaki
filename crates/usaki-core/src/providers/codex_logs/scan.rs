//! Finds the newest rate limit event in a Codex sessions folder.
//!
//! Codex writes one `rollout-*.jsonl` file per session under `sessions/YYYY/MM/DD`. A resumed
//! session appends to its old file, so the scan orders the files by their modified time and
//! not by their folder. It reads each file from the end, so the newest event comes first.
//! Older sessions are `.jsonl.zst` files. The scan skips them.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::SystemTime;

use chrono::{DateTime, Utc};

use super::event::{CodexEvent, parse_line};

/// The extension of a session log that the scan reads.
const LOG_EXTENSION: &str = "jsonl";
/// The scan reads a file backwards in blocks of this size.
const CHUNK: usize = 64 * 1024;
/// A line longer than this cannot be a `token_count` line. The scan skips it unparsed.
const MAX_LINE: usize = 1024 * 1024;
/// The deepest folder level that the scan enters. Codex uses three: year, month and day.
const MAX_DEPTH: usize = 6;
/// The most files that one scan reads. Files that the cache holds do not count.
const MAX_READS: usize = 64;

/// The newest event, with the time it was written.
#[derive(Debug, Clone, PartialEq)]
pub struct Latest {
    /// The `timestamp` of the line, else the modified time of the file.
    pub at: DateTime<Utc>,
    pub event: CodexEvent,
}

#[derive(Debug, Clone)]
struct Entry {
    len: u64,
    modified: SystemTime,
    event: Option<CodexEvent>,
}

/// The last event of each file, keyed by the path. A file whose size and modified time
/// did not change keeps its entry, so a poll does not read it again.
#[derive(Debug, Default)]
pub struct ScanCache(Mutex<BTreeMap<PathBuf, Entry>>);

struct LogFile {
    path: PathBuf,
    len: u64,
    modified: SystemTime,
}

/// Returns the newest rate limit event under `sessions`, or `None` when no file holds one.
///
/// A missing `sessions` folder gives a `NotFound` error. A subfolder that cannot be read is
/// skipped.
pub fn newest_event(sessions: &Path, cache: &ScanCache) -> io::Result<Option<Latest>> {
    let mut files = Vec::new();
    walk(sessions, 0, &mut files, true)?;
    files.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| b.path.cmp(&a.path))
    });

    let mut entries = cache.0.lock().unwrap_or_else(PoisonError::into_inner);
    entries.retain(|path, _| files.iter().any(|f| &f.path == path));
    let mut best: Option<Latest> = None;
    let mut reads = 0;
    for file in &files {
        let modified = DateTime::<Utc>::from(file.modified);
        // A file holds no event newer than its last write.
        if best.as_ref().is_some_and(|b| modified < b.at) {
            break;
        }
        let cached = entries
            .get(&file.path)
            .filter(|e| e.len == file.len && e.modified == file.modified);
        let event = match cached {
            Some(entry) => entry.event.clone(),
            None => {
                if reads == MAX_READS {
                    break;
                }
                reads += 1;
                let Ok(event) = last_event(&file.path) else {
                    continue;
                };
                entries.insert(
                    file.path.clone(),
                    Entry {
                        len: file.len,
                        modified: file.modified,
                        event: event.clone(),
                    },
                );
                event
            }
        };
        if let Some(event) = event {
            let at = event.at.unwrap_or(modified);
            if best.as_ref().is_none_or(|b| at > b.at) {
                best = Some(Latest { at, event });
            }
        }
    }
    Ok(best)
}

// Collects the session logs under `dir`. The scan does not follow a symbolic link, so a
// link loop cannot trap it. Only the root reports an error.
fn walk(dir: &Path, depth: usize, out: &mut Vec<LogFile>, root: bool) -> io::Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if root => return Err(e),
        Err(_) => return Ok(()),
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() && depth < MAX_DEPTH {
            walk(&path, depth + 1, out, false)?;
        } else if kind.is_file() && path.extension().is_some_and(|x| x == LOG_EXTENSION) {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = meta.modified() else {
                continue;
            };
            out.push(LogFile {
                path,
                len: meta.len(),
                modified,
            });
        }
    }
    Ok(())
}

/// Returns the last rate limit event of one file. It reads the file from the end.
pub fn last_event(path: &Path) -> io::Result<Option<CodexEvent>> {
    let mut file = File::open(path)?;
    let mut pos = file.metadata()?.len();
    // The front part of a line whose end the scan already read.
    let mut tail: Vec<u8> = Vec::new();
    // True while the scan is in a line longer than MAX_LINE.
    let mut skipping = false;
    loop {
        let n = usize::try_from(pos).map_or(CHUNK, |p| p.min(CHUNK));
        pos -= n as u64;
        file.seek(SeekFrom::Start(pos))?;
        let mut buf = vec![0; n];
        file.read_exact(&mut buf)?;
        buf.extend_from_slice(&tail);
        let mut end = buf.len();
        while let Some(i) = buf[..end].iter().rposition(|&b| b == b'\n') {
            if skipping {
                skipping = false;
            } else if let Some(event) = parse_line(&buf[i + 1..end]) {
                return Ok(Some(event));
            }
            end = i;
        }
        buf.truncate(end);
        if pos == 0 {
            return Ok(if skipping { None } else { parse_line(&buf) });
        }
        if buf.len() > MAX_LINE {
            tail = Vec::new();
            skipping = true;
        } else {
            tail = buf;
        }
    }
}
