//! Hub mode. It resolves the mode and keeps the hub state between polls.
//!
//! The state holds the endpoint throttle, the last endpoint reply and the rate history, so it
//! lives as long as the app. See `docs/design/claude-hub.md`.

use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};

use usaki_core::providers::claude_hub::{HubDirs, HubReader, StatusClient, UreqStatusClient};

use crate::hub_view::{PoolView, UsakiMode};

/// The environment variable that sets the mode.
pub const MODE_ENV: &str = "USAKI_MODE";
/// The environment variable that replaces the config folder.
pub const CONFIG_DIR_ENV: &str = "USAKI_CONFIG_DIR";

/// The mode that the user asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeSetting {
    Auto,
    Normal,
    Hub,
}

impl ModeSetting {
    /// Parses `auto`, `normal` or `hub`. Anything else is `None`.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "normal" => Some(Self::Normal),
            "hub" => Some(Self::Hub),
            _ => None,
        }
    }
}

/// Resolves the setting. The env value wins, then the `mode` key of the config file, then auto.
/// A bad value falls back to auto.
pub fn resolve_setting(env: Option<&str>, config_dir: Option<&Path>) -> ModeSetting {
    if let Some(value) = env.filter(|v| !v.is_empty()) {
        return ModeSetting::parse(value).unwrap_or(ModeSetting::Auto);
    }
    config_dir
        .and_then(|dir| std::fs::read_to_string(dir.join("config.toml")).ok())
        .and_then(|text| text.parse::<toml::Table>().ok())
        .and_then(|table| table.get("mode")?.as_str().and_then(ModeSetting::parse))
        .unwrap_or(ModeSetting::Auto)
}

type Reader = HubReader<Box<dyn StatusClient + Send>>;

/// The result of one hub read.
#[derive(Debug, Clone, PartialEq)]
pub struct HubRead {
    pub mode: UsakiMode,
    pub pools: Vec<PoolView>,
    /// The seat names of the pools that have data. Direct Claude accounts with these names leave
    /// the account list.
    pub seats: HashSet<String>,
}

impl HubRead {
    fn normal() -> Self {
        Self {
            mode: UsakiMode::Normal,
            pools: Vec::new(),
            seats: HashSet::new(),
        }
    }
}

/// The hub state. Clones share one reader.
#[derive(Clone)]
pub struct HubState {
    setting: ModeSetting,
    reader: Option<Arc<Mutex<Reader>>>,
}

impl HubState {
    /// Builds the state from the environment.
    pub fn from_env() -> Self {
        let env = std::env::var(MODE_ENV).ok();
        let conf = std::env::var(CONFIG_DIR_ENV)
            .ok()
            .filter(|v| !v.is_empty())
            .map(std::path::PathBuf::from)
            .or_else(|| dirs::config_dir().map(|d| d.join("usaki")));
        let setting = resolve_setting(env.as_deref(), conf.as_deref());
        match HubDirs::from_env() {
            Ok(dirs) => Self::new(setting, dirs, Box::new(UreqStatusClient::new())),
            Err(e) => {
                eprintln!("usaki: {e}");
                Self {
                    setting,
                    reader: None,
                }
            }
        }
    }

    pub fn new(setting: ModeSetting, dirs: HubDirs, client: Box<dyn StatusClient + Send>) -> Self {
        Self {
            setting,
            reader: Some(Arc::new(Mutex::new(HubReader::new(dirs, client)))),
        }
    }

    /// A state that never reads a pool. Tests of the normal mode use it.
    #[cfg(test)]
    pub fn normal() -> Self {
        Self {
            setting: ModeSetting::Normal,
            reader: None,
        }
    }

    /// Reads the pools at `now` (epoch seconds) and resolves the mode.
    ///
    /// `normal` never reads a pool. `auto` is hub when at least one pool exists. `hub` is always
    /// hub, with no pools when none exist.
    pub fn read(&self, now: i64) -> HubRead {
        if self.setting == ModeSetting::Normal {
            return HubRead::normal();
        }
        let readings = match &self.reader {
            Some(reader) => reader
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .poll(now),
            None => Vec::new(),
        };
        if readings.is_empty() && self.setting == ModeSetting::Auto {
            return HubRead::normal();
        }
        let seats = readings
            .iter()
            .filter_map(|r| r.status.as_ref())
            .flat_map(|s| s.seats.iter().map(|seat| seat.seat.clone()))
            .collect();
        HubRead {
            mode: UsakiMode::Hub,
            pools: readings
                .iter()
                .map(|r| PoolView::from_reading(r, now))
                .collect(),
            seats,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conf(text: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.toml"), text).unwrap();
        dir
    }

    #[test]
    fn parses_the_three_values() {
        assert_eq!(ModeSetting::parse("auto"), Some(ModeSetting::Auto));
        assert_eq!(ModeSetting::parse(" Hub "), Some(ModeSetting::Hub));
        assert_eq!(ModeSetting::parse("normal"), Some(ModeSetting::Normal));
        assert_eq!(ModeSetting::parse("pool"), None);
    }

    #[test]
    fn env_wins_over_the_config_file() {
        let dir = conf("mode = \"normal\"\n");
        assert_eq!(
            resolve_setting(Some("hub"), Some(dir.path())),
            ModeSetting::Hub
        );
    }

    #[test]
    fn config_file_sets_the_mode_without_env() {
        let dir = conf("mode = \"hub\"\n[other]\nmode = \"normal\"\n");
        assert_eq!(resolve_setting(None, Some(dir.path())), ModeSetting::Hub);
        assert_eq!(
            resolve_setting(Some(""), Some(dir.path())),
            ModeSetting::Hub
        );
    }

    #[test]
    fn a_bad_value_falls_back_to_auto() {
        let dir = conf("mode = \"pool\"\n");
        assert_eq!(resolve_setting(None, Some(dir.path())), ModeSetting::Auto);
        assert_eq!(
            resolve_setting(Some("pool"), Some(dir.path())),
            ModeSetting::Auto
        );
        let bad = conf("mode = = =");
        assert_eq!(resolve_setting(None, Some(bad.path())), ModeSetting::Auto);
    }

    #[test]
    fn missing_config_gives_auto() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(resolve_setting(None, Some(dir.path())), ModeSetting::Auto);
        assert_eq!(resolve_setting(None, None), ModeSetting::Auto);
    }
}
