//! Claude watcher provider. It reads the usage of a watcher login.
//!
//! A watcher login is a normal Claude Code login in a config dir that only usaKi uses:
//! `<data dir>/usaki/watch/<acct>`. No working session shares its token chain. The provider
//! reads the access token read-only and calls the read-only usage API with it. When the token
//! is about to expire, the provider asks the official `claude` binary to refresh it in the
//! watcher dir. usaKi itself never refreshes a token and never writes a credentials file.
//!
//! The design is in `docs/design/watcher-logins.md`.

pub mod cli;
pub mod credentials;
pub mod http;
pub mod usage;
pub mod watcher;

use std::io;
use std::path::{Path, PathBuf};

use chrono::Duration;

use super::claude_cache::is_valid_account_id;
use crate::error::{Error, Result};

pub use cli::{Busy, ClaudeBinary, ClaudeCli, RunEnd};
pub use credentials::{AccessToken, Credentials, parse_credentials, read_credentials};
pub use http::{HttpReply, UreqClient, UsageClient};
pub use usage::{UsageReply, classify_reply, parse_usage_body};
pub use watcher::{Problem, Reading, WatchRecord, Watcher};

/// The read-only usage endpoint.
pub const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
/// The value of the `anthropic-beta` header that the usage endpoint needs.
pub const OAUTH_BETA: &str = "oauth-2025-04-20";
/// The file in a watcher dir that holds the token. The official binary writes it.
pub const CREDENTIALS_FILE: &str = ".credentials.json";

/// The normal time between two usage calls for one account.
pub const CALL_INTERVAL: Duration = Duration::minutes(5);
/// The longest time between two usage calls after repeated 429 replies.
pub const MAX_CALL_INTERVAL: Duration = Duration::minutes(60);
/// The provider asks for a refresh when the token expires within this time.
pub const REFRESH_LEAD: Duration = Duration::minutes(2);
/// A reading older than this is stale.
pub const STALE_AFTER: Duration = Duration::minutes(15);

/// Returns the watch root under a data directory: `<data dir>/usaki/watch`.
pub fn watch_root(data_dir: &Path) -> PathBuf {
    data_dir.join("usaki").join("watch")
}

/// Returns the watch root from the platform data directory.
///
/// On Linux the data directory is XDG_DATA_HOME, else `~/.local/share`.
pub fn watch_root_from_env() -> Result<PathBuf> {
    dirs::data_dir()
        .map(|d| watch_root(&d))
        .ok_or(Error::NoDataDir)
}

/// Returns the watcher dir of an account. The caller must check the id first.
pub fn watch_dir(root: &Path, account_id: &str) -> PathBuf {
    root.join(account_id)
}

/// Returns the credentials file of a watcher dir.
pub fn credentials_path(dir: &Path) -> PathBuf {
    dir.join(CREDENTIALS_FILE)
}

/// Returns the id of each watcher login under `root`, sorted.
///
/// Each subdir whose name passes `is_valid_account_id` is one account. A missing root gives
/// no accounts.
pub fn discover_accounts(root: &Path) -> io::Result<Vec<String>> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut ids: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .filter(|id| is_valid_account_id(id))
        .collect();
    ids.sort();
    Ok(ids)
}
