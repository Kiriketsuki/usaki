//! Claude hub provider. It reads the status of the pools of a Claude hub.
//!
//! A pool is a group of seats (Claude accounts) behind one hub. The status comes from the
//! cache file that the hub statusline keeps, or from the status endpoint of the pool as a
//! fallback. The contract is in `docs/design/claude-hub.md`.

pub mod forecast;
pub mod http;
pub mod live;
pub mod source;
pub mod status;

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::providers::claude_cache::cache_dir_from_env;

pub use forecast::{Basis, Forecast, PoolTotal, RateHistory, Sample};
pub use http::{StatusClient, UreqStatusClient};
pub use source::{DataSource, HubReader, PoolReading};
pub use status::{PoolStatus, SeatState, SeatStatus, WindowStatus, parse_status};

/// The suffix of the status cache file: `<pool>.status.json`.
pub const STATUS_FILE_SUFFIX: &str = ".status.json";
/// The suffix of the file that holds the status URL: `<pool>.status-url`.
pub const STATUS_URL_SUFFIX: &str = ".status-url";
/// The suffix of the file that holds the API key: `<pool>.key`.
pub const KEY_SUFFIX: &str = ".key";
/// The suffix of the file that holds the pool URL: `<pool>.url`.
pub const POOL_URL_SUFFIX: &str = ".url";
/// The suffix of the folder that holds the live session files: `<pool>.live`.
pub const LIVE_DIR_SUFFIX: &str = ".live";

/// Returns true when `name` is a valid pool name: `[a-z0-9-]+`.
pub fn is_valid_pool_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The two folders that the hub uses: the config folder and the cache folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubDirs {
    pub conf: PathBuf,
    pub cache: PathBuf,
}

impl HubDirs {
    pub fn new(conf: impl Into<PathBuf>, cache: impl Into<PathBuf>) -> Self {
        Self {
            conf: conf.into(),
            cache: cache.into(),
        }
    }

    /// Resolves the folders from the environment.
    ///
    /// The config folder is `$CLIPROXY_CONF_DIR`, else `~/.config/cliproxy`. The cache folder is
    /// `$CLIPROXY_CACHE_DIR`, else `<cache base>/cliproxy`.
    pub fn from_env() -> Result<Self> {
        let non_empty = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        let conf = match non_empty("CLIPROXY_CONF_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => dirs::home_dir()
                .ok_or(Error::NoCacheDir)?
                .join(".config")
                .join("cliproxy"),
        };
        let cache = match non_empty("CLIPROXY_CACHE_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => cache_dir_from_env()?.join("cliproxy"),
        };
        Ok(Self { conf, cache })
    }

    /// Returns the sorted names of the pools that exist.
    ///
    /// A pool exists when `<cache>/<pool>.status.json` exists, or when both
    /// `<conf>/<pool>.status-url` and `<conf>/<pool>.key` exist.
    pub fn discover_pools(&self) -> Vec<String> {
        let mut names: Vec<String> = file_stems(&self.cache, STATUS_FILE_SUFFIX);
        let urls = file_stems(&self.conf, STATUS_URL_SUFFIX);
        names.extend(
            urls.into_iter()
                .filter(|n| self.conf.join(format!("{n}{KEY_SUFFIX}")).is_file()),
        );
        names.sort();
        names.dedup();
        names
    }

    /// Returns the path of a file of a pool in the cache folder.
    pub fn cache_file(&self, pool: &str, suffix: &str) -> PathBuf {
        self.cache.join(format!("{pool}{suffix}"))
    }

    /// Returns the path of a file of a pool in the config folder.
    pub fn conf_file(&self, pool: &str, suffix: &str) -> PathBuf {
        self.conf.join(format!("{pool}{suffix}"))
    }
}

/// Returns the valid pool names of the files in `dir` that end with `suffix`.
fn file_stems(dir: &Path, suffix: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_owned();
            let stem = name.strip_suffix(suffix)?;
            is_valid_pool_name(stem).then(|| stem.to_owned())
        })
        .collect()
}
