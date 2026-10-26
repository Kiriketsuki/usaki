//! Core library for usaKi. It holds the usage model and the data providers.

pub mod error;
pub mod model;
pub mod providers;

pub use error::{Error, Result};
pub use model::{Credits, Snapshot, Source, Window, WindowKind, kind_from_minutes};
pub use providers::claude_cache::ClaudeCacheProvider;
pub use providers::codex_logs::CodexLogsProvider;
pub use providers::{Account, Clock, Provider, ProviderKind, SystemClock};
