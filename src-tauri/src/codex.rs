//! Codex account discovery and reads.
//!
//! The app has no config file yet. The Codex home from the environment is one account, named
//! `DEFAULT_CODEX_NAME`, when its sessions folder exists. The config file in milestone 3 adds
//! more homes, so the sources hold a list.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use usaki_core::providers::codex_logs::{ScanCache, codex_home_from_env, sessions_dir};
use usaki_core::{Account, Clock, CodexLogsProvider, Provider, ProviderKind};

use crate::view::{AccountView, ProviderName};

/// The name of the account of the default Codex home. A plan type would change with the
/// plan and drop a pin, so the name is fixed.
pub const DEFAULT_CODEX_NAME: &str = "codex";

/// One Codex home and the name of its account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexHome {
    pub name: String,
    pub home: PathBuf,
}

/// The Codex homes to read, and the scan cache that the polls share.
#[derive(Debug, Clone, Default)]
pub struct CodexSources {
    homes: Vec<CodexHome>,
    cache: Arc<ScanCache>,
}

impl CodexSources {
    pub fn new(homes: Vec<CodexHome>) -> Self {
        Self {
            homes,
            cache: Arc::default(),
        }
    }

    /// Returns the default Codex home from CODEX_HOME, else `~/.codex`. Without a home
    /// directory the app runs with no Codex account.
    pub fn from_env() -> Self {
        match codex_home_from_env() {
            Ok(home) => Self::new(vec![CodexHome {
                name: DEFAULT_CODEX_NAME.to_owned(),
                home,
            }]),
            Err(e) => {
                eprintln!("usaki: {e} The app does not read Codex logs.");
                Self::default()
            }
        }
    }

    /// Reads each home that has a sessions folder, sorted by name. A home without one is
    /// not a Codex install, so it gives no account.
    pub fn read<C: Clock + Clone>(&self, clock: C, now: DateTime<Utc>) -> Vec<AccountView> {
        let provider = CodexLogsProvider::new(clock).with_cache(self.cache.clone());
        let mut homes: Vec<&CodexHome> = self
            .homes
            .iter()
            .filter(|h| sessions_dir(&h.home).is_dir())
            .collect();
        homes.sort_by(|a, b| a.name.cmp(&b.name));
        homes
            .into_iter()
            .map(|h| {
                let acct = Account::new(&h.name, ProviderKind::CodexLogs, Some(h.home.clone()));
                AccountView::from_result(ProviderName::Codex, &h.name, provider.fetch(&acct), now)
            })
            .collect()
    }
}
