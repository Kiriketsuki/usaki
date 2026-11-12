//! The source rule. A fresh cache file wins. Else the reader calls the status endpoint, at
//! most once per 60 s for each pool. Else it uses the newest data that it has.

use std::collections::{BTreeMap, HashMap};

use super::forecast::{self, Forecast, PoolTotal, RateHistory};
use super::http::StatusClient;
use super::live::read_live;
use super::status::{PoolStatus, parse_status};
use super::{HubDirs, KEY_SUFFIX, POOL_URL_SUFFIX, STATUS_FILE_SUFFIX, STATUS_URL_SUFFIX};

/// A cache file at most this old is fresh, in seconds.
pub const FRESH_SECS: i64 = 120;
/// The least time between two endpoint calls for one pool, in seconds.
pub const CALL_INTERVAL_SECS: i64 = 60;
/// Data older than this is stale, in seconds.
pub const STALE_SECS: i64 = 600;

/// Where the data of a pool came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSource {
    Cache,
    Endpoint,
}

/// The reading of one pool.
#[derive(Debug, Clone, PartialEq)]
pub struct PoolReading {
    pub name: String,
    pub source: DataSource,
    /// `None` when the pool has no readable data.
    pub status: Option<PoolStatus>,
    pub updated_epoch: i64,
    pub fetched_at: i64,
    pub stale: bool,
    pub error: Option<String>,
    /// Live sessions for each seat. `None` when the live folder does not exist.
    pub live: Option<BTreeMap<String, u32>>,
    pub total: Option<PoolTotal>,
    pub forecast: Option<Forecast>,
}

/// Reads the pools. It keeps the throttle, the last endpoint reply and the rate history.
pub struct HubReader<C> {
    dirs: HubDirs,
    client: C,
    last_call: HashMap<String, i64>,
    last_reply: HashMap<String, PoolStatus>,
    history: RateHistory,
}

impl<C: StatusClient> HubReader<C> {
    pub fn new(dirs: HubDirs, client: C) -> Self {
        Self {
            dirs,
            client,
            last_call: HashMap::new(),
            last_reply: HashMap::new(),
            history: RateHistory::default(),
        }
    }

    pub fn dirs(&self) -> &HubDirs {
        &self.dirs
    }

    /// Reads every pool that exists, sorted by name.
    pub fn poll(&mut self, now: i64) -> Vec<PoolReading> {
        self.dirs
            .discover_pools()
            .into_iter()
            .map(|name| self.read_pool(&name, now))
            .collect()
    }

    /// Reads one pool by the source rule.
    pub fn read_pool(&mut self, name: &str, now: i64) -> PoolReading {
        let (cache, mut error) = match self.read_cache(name) {
            Ok(status) => (status, None),
            Err(e) => (None, Some(e)),
        };
        let fresh = cache
            .as_ref()
            .is_some_and(|s| now - s.updated_epoch <= FRESH_SECS);
        if !fresh && let Some(e) = self.try_endpoint(name, now) {
            error = Some(e);
        }
        let reply = self.last_reply.get(name).cloned();
        let (status, source) = match (cache, reply) {
            (Some(c), Some(r)) if c.updated_epoch > r.updated_epoch => (Some(c), DataSource::Cache),
            (_, Some(r)) => (Some(r), DataSource::Endpoint),
            (Some(c), None) => (Some(c), DataSource::Cache),
            (None, None) => (None, DataSource::Cache),
        };
        self.assemble(name, now, status, source, error)
    }

    fn assemble(
        &mut self,
        name: &str,
        now: i64,
        status: Option<PoolStatus>,
        source: DataSource,
        error: Option<String>,
    ) -> PoolReading {
        let Some(status) = status else {
            let reason = error.unwrap_or_else(|| "The pool has no data.".to_owned());
            return PoolReading {
                name: name.to_owned(),
                source,
                status: None,
                updated_epoch: now,
                fetched_at: now,
                stale: true,
                error: Some(reason),
                live: None,
                total: None,
                forecast: None,
            };
        };
        let stale = now - status.updated_epoch > STALE_SECS;
        let error = stale
            .then(|| error.unwrap_or_else(|| "The pool data is older than 10 minutes.".to_owned()));
        self.history.record(name, &status, now);
        let samples = self.history.samples(name);
        PoolReading {
            name: name.to_owned(),
            source,
            updated_epoch: status.updated_epoch,
            fetched_at: status.fetched_at.unwrap_or(status.updated_epoch),
            stale,
            error,
            live: read_live(&self.dirs, name, now),
            total: Some(forecast::pool_total(&status, now)),
            forecast: forecast::forecast(&status, &samples, now),
            status: Some(status),
        }
    }

    /// Reads the cache file. A missing file is `Ok(None)`.
    fn read_cache(&self, name: &str) -> Result<Option<PoolStatus>, String> {
        let path = self.dirs.cache_file(name, STATUS_FILE_SUFFIX);
        match std::fs::read_to_string(&path) {
            Ok(text) => parse_status(&text).map(Some),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("Cannot read the file {}. {e}", path.display())),
        }
    }

    /// Calls the endpoint when the config exists and the throttle allows it.
    ///
    /// Returns the error text when the call is refused or fails.
    fn try_endpoint(&mut self, name: &str, now: i64) -> Option<String> {
        let (url, key) = match self.endpoint_config(name)? {
            Ok(cfg) => cfg,
            Err(e) => return Some(e),
        };
        if self
            .last_call
            .get(name)
            .is_some_and(|t| now - t < CALL_INTERVAL_SECS)
        {
            return None;
        }
        self.last_call.insert(name.to_owned(), now);
        let result = self
            .client
            .get_status(&url, &key)
            .and_then(|reply| match reply.status {
                200 => parse_status(&reply.body),
                code => Err(format!("The status endpoint replied HTTP {code}.")),
            });
        match result {
            Ok(mut status) => {
                status.fetched_at = Some(now);
                self.last_reply.insert(name.to_owned(), status);
                None
            }
            Err(e) => Some(e.replace(&key, "[key]")),
        }
    }

    /// Reads the endpoint config. `None` when the files are missing.
    fn endpoint_config(&self, name: &str) -> Option<Result<(String, String), String>> {
        let first_line = |suffix: &str| {
            let text = std::fs::read_to_string(self.dirs.conf_file(name, suffix)).ok()?;
            let line = text.lines().next()?.trim().to_owned();
            (!line.is_empty()).then_some(line)
        };
        let url = first_line(STATUS_URL_SUFFIX)?;
        let key = first_line(KEY_SUFFIX)?;
        let pool_url = first_line(POOL_URL_SUFFIX)?;
        Some(match (https_host(&url), url_host(&pool_url)) {
            (Some(a), Some(b)) if a == b => Ok((url, key)),
            _ => {
                Err("The status URL is not https or its host differs from the pool URL.".to_owned())
            }
        })
    }
}

/// Returns the lower case host of a URL, or `None` when the URL has no valid authority.
fn url_host(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.contains('@') {
        return None;
    }
    let host = authority.rsplit_once(':').map_or(authority, |(h, _)| h);
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// Like `url_host`, and `None` unless the scheme is https.
fn https_host(url: &str) -> Option<String> {
    url.get(..8)
        .is_some_and(|s| s.eq_ignore_ascii_case("https://"))
        .then(|| url_host(url))?
}
