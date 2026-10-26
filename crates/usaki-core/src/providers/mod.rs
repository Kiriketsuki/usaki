//! Data providers. Each provider turns one source into a `Snapshot`.

pub mod claude_cache;
pub mod claude_watch;
pub mod codex_logs;

use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::error::Result;
use crate::model::Snapshot;

/// The kind of provider that serves an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    ClaudeCache,
    /// The Codex session logs under one Codex home.
    CodexLogs,
}

/// A configured account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: String,
    pub provider: ProviderKind,
    /// Overrides the default source location when set.
    pub source: Option<PathBuf>,
}

impl Account {
    pub fn new(id: impl Into<String>, provider: ProviderKind, source: Option<PathBuf>) -> Self {
        Self {
            id: id.into(),
            provider,
            source,
        }
    }
}

/// A source of the current time. Tests inject a fixed clock.
pub trait Clock {
    fn now(&self) -> DateTime<Utc>;
}

/// The real clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// A source of usage snapshots.
pub trait Provider {
    fn fetch(&self, acct: &Account) -> Result<Snapshot>;
}
