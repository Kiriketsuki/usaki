//! The watcher. It keeps the schedule and the last reading of each watcher login.
//!
//! Rate rules:
//! - At most one usage call per account per `CALL_INTERVAL`.
//! - A 429 reply doubles the interval, up to `MAX_CALL_INTERVAL`. A good reply resets it.
//! - A 401 reply or an expired token makes the account stale, with a reason.
//!
//! Refresh rule: when the token expires within `REFRESH_LEAD`, the watcher runs
//! `claude auth status --json` in the watcher dir. When the expiry does not change, it runs
//! `claude -p "/usage"`. A refresh uses the call slot of the account. While a refresh child of
//! an account has not exited, the watcher makes no refresh and no API call for that account.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};

use super::cli::{Busy, ClaudeCli, RunEnd};
use super::credentials::{Credentials, read_credentials};
use super::http::UsageClient;
use super::usage::{UsageReply, classify_reply};
use super::{
    CALL_INTERVAL, MAX_CALL_INTERVAL, REFRESH_LEAD, STALE_AFTER, credentials_path,
    discover_accounts, watch_dir,
};
use crate::model::{Snapshot, Source, Window};
use crate::providers::Clock;

/// The longest time that `claude auth status --json` may take.
pub const AUTH_STATUS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
/// The longest time that `claude -p "/usage"` may take.
pub const PRINT_USAGE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(180);

/// The time after which the watcher reads the `claude` version again for the User-Agent.
pub const VERSION_RECHECK: Duration = Duration::hours(1);

const AUTH_STATUS_ARGS: [&str; 3] = ["auth", "status", "--json"];
const PRINT_USAGE_ARGS: [&str; 2] = ["-p", "/usage"];

/// Why the watcher has no fresh reading for an account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The watcher dir has no credentials file.
    NoCredentials,
    /// The credentials file does not parse. The value is the error text.
    BadCredentials(String),
    /// The token has expired and the refresh did not give a new one.
    Expired(String),
    /// A refresh child of the watcher dir has not exited yet.
    RefreshRunning,
    /// The API replied 401.
    Unauthorized(Option<String>),
    /// The API replied 429.
    RateLimited,
    /// The API replied with another status.
    Http {
        status: u16,
        message: Option<String>,
    },
    /// The call did not complete.
    Transport(String),
    /// The API replied 200 with a body that does not parse.
    BadReply(String),
}

impl Problem {
    /// Returns true when the problem makes any old reading stale at once.
    fn ends_reading(&self) -> bool {
        matches!(
            self,
            Self::NoCredentials
                | Self::BadCredentials(_)
                | Self::Expired(_)
                | Self::Unauthorized(_)
        )
    }

    /// Returns the reason in strict STE form.
    pub fn reason(&self) -> String {
        let with = |base: &str, extra: &Option<String>| match extra {
            Some(m) => format!("{base} {m}"),
            None => base.to_owned(),
        };
        match self {
            Self::NoCredentials => {
                "The watcher login has no credentials file. Run claude auth login in it.".into()
            }
            Self::BadCredentials(e) => format!("Cannot read the watcher credentials. {e}"),
            Self::Expired(why) => format!("The watcher token has expired. {why}"),
            Self::RefreshRunning => {
                "A claude refresh still runs in the watcher dir. usaKi waits for its exit.".into()
            }
            Self::Unauthorized(m) => with("The usage API rejected the token (HTTP 401).", m),
            Self::RateLimited => "The usage API limits the calls (HTTP 429). usaKi waits.".into(),
            Self::Http { status, message } => {
                with(&format!("The usage API replied HTTP {status}."), message)
            }
            Self::Transport(e) => e.clone(),
            Self::BadReply(e) => e.clone(),
        }
    }
}

/// The windows of one good usage reply.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    pub windows: Vec<Window>,
    pub fetched_at: DateTime<Utc>,
}

/// What the watcher knows about one account.
#[derive(Debug, Clone, PartialEq)]
pub struct WatchRecord {
    pub account_id: String,
    /// The last good reading, if any.
    pub reading: Option<Reading>,
    /// The problem of the last attempt, if it failed.
    pub problem: Option<Problem>,
}

impl WatchRecord {
    fn new(account_id: &str) -> Self {
        Self {
            account_id: account_id.to_owned(),
            reading: None,
            problem: None,
        }
    }

    /// Returns the snapshot of this record as it stands at `now`.
    ///
    /// A record is stale when it has no reading, when the reading is older than
    /// `STALE_AFTER`, or when the last problem ends the reading (401, expired token, no
    /// credentials). A 429 or a network failure keeps a recent reading fresh.
    pub fn snapshot(&self, now: DateTime<Utc>) -> Snapshot {
        let (windows, fetched_at) = match &self.reading {
            Some(r) => (
                r.windows.iter().map(|w| w.as_of(now)).collect(),
                r.fetched_at,
            ),
            None => (Vec::new(), now),
        };
        let too_old = self
            .reading
            .as_ref()
            .is_none_or(|r| now - r.fetched_at > STALE_AFTER);
        let ended = self.problem.as_ref().is_some_and(Problem::ends_reading);
        let snap = Snapshot::new(
            &self.account_id,
            windows,
            fetched_at,
            Source::ClaudeWatch,
            too_old || ended,
        );
        if !snap.stale {
            return snap;
        }
        let reason = match (&self.problem, &self.reading) {
            (Some(p), _) => p.reason(),
            (None, None) => "The watcher has no reading yet.".to_owned(),
            (None, Some(_)) => "The last watcher reading is older than 15 minutes.".to_owned(),
        };
        snap.with_stale_reason(reason)
    }
}

#[derive(Debug, Clone)]
struct AccountState {
    record: WatchRecord,
    next_call: Option<DateTime<Utc>>,
    interval: Duration,
    /// Set while a refresh child of this account has not exited.
    busy: Busy,
}

/// The result of one attempt. `used_slot` is true when the attempt called the API or the binary.
struct Attempt {
    used_slot: bool,
    result: Result<Vec<Window>, Problem>,
}

/// Polls every watcher login under one root, and keeps to the rate rules.
pub struct Watcher<C: Clock, H: UsageClient, L: ClaudeCli> {
    root: PathBuf,
    clock: C,
    http: H,
    cli: Option<L>,
    user_agent: Option<String>,
    version_checked: Option<DateTime<Utc>>,
    auth_status_timeout: std::time::Duration,
    print_usage_timeout: std::time::Duration,
    accounts: BTreeMap<String, AccountState>,
}

impl<C: Clock, H: UsageClient, L: ClaudeCli> Watcher<C, H, L> {
    /// `cli` is `None` when the `claude` binary is not found. The watcher then cannot refresh.
    pub fn new(root: PathBuf, clock: C, http: H, cli: Option<L>) -> Self {
        Self {
            root,
            clock,
            http,
            cli,
            user_agent: None,
            version_checked: None,
            auth_status_timeout: AUTH_STATUS_TIMEOUT,
            print_usage_timeout: PRINT_USAGE_TIMEOUT,
            accounts: BTreeMap::new(),
        }
    }

    /// Returns a watcher with other refresh timeouts. Tests use short ones.
    pub fn with_timeouts(
        self,
        auth_status: std::time::Duration,
        print_usage: std::time::Duration,
    ) -> Self {
        Self {
            auth_status_timeout: auth_status,
            print_usage_timeout: print_usage,
            ..self
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Finds the accounts and polls each one that is due.
    ///
    /// `skip` returns true for an account that has a fresh statusline cache. The watcher then
    /// makes no call for it. The result holds every account under the root, sorted by id.
    pub fn tick(&mut self, skip: impl Fn(&str) -> bool) -> io::Result<Vec<WatchRecord>> {
        let ids = discover_accounts(&self.root)?;
        self.accounts.retain(|id, _| ids.contains(id));
        for id in &ids {
            self.accounts
                .entry(id.clone())
                .or_insert_with(|| AccountState {
                    record: WatchRecord::new(id),
                    next_call: None,
                    interval: CALL_INTERVAL,
                    busy: Busy::default(),
                });
            if !skip(id) {
                self.poll(id);
            }
        }
        Ok(self.records())
    }

    /// Returns the record of every known account, sorted by id.
    pub fn records(&self) -> Vec<WatchRecord> {
        self.accounts.values().map(|s| s.record.clone()).collect()
    }

    /// Returns true while a refresh child of the account has not exited.
    pub fn is_busy(&self, id: &str) -> bool {
        self.accounts.get(id).is_some_and(|s| s.busy.is_set())
    }

    /// Polls one known account when its call slot is due and no refresh child runs.
    fn poll(&mut self, id: &str) {
        let now = self.clock.now();
        let due = self
            .accounts
            .get(id)
            .is_some_and(|s| !s.busy.is_set() && s.next_call.is_none_or(|t| now >= t));
        if !due {
            return;
        }
        let attempt = self.attempt(id, now);
        // A refresh can take minutes, so the schedule starts after the attempt.
        let done = self.clock.now();
        if let Some(state) = self.accounts.get_mut(id) {
            *state = apply(state.clone(), attempt, done);
        }
    }

    fn attempt(&mut self, id: &str, now: DateTime<Utc>) -> Attempt {
        let busy = self
            .accounts
            .get(id)
            .map(|s| s.busy.clone())
            .unwrap_or_default();
        let dir = watch_dir(&self.root, id);
        let path = credentials_path(&dir);
        let creds = match read_now(&path) {
            Ok(creds) => creds,
            Err(problem) => {
                return Attempt {
                    used_slot: false,
                    result: Err(problem),
                };
            }
        };
        let creds = if creds.expires_at - now <= REFRESH_LEAD {
            match self.refresh(&dir, &path, creds, &busy) {
                // A child past its timeout can still use the refresh token. The API call
                // waits until the reaper clears the flag.
                Ok(_) if busy.is_set() => {
                    return Attempt {
                        used_slot: true,
                        result: Err(Problem::RefreshRunning),
                    };
                }
                Ok(creds) => creds,
                Err(problem) => {
                    return Attempt {
                        used_slot: true,
                        result: Err(problem),
                    };
                }
            }
        } else {
            creds
        };
        let user_agent = self.user_agent();
        let result = match self.http.fetch_usage(&creds.token, &user_agent) {
            Err(e) => Err(Problem::Transport(e)),
            Ok(reply) => match classify_reply(reply.status, &reply.body) {
                UsageReply::Usage(windows) => Ok(windows),
                UsageReply::Unauthorized(m) => Err(Problem::Unauthorized(m)),
                UsageReply::RateLimited(_) => Err(Problem::RateLimited),
                UsageReply::Failed { status, message } => Err(Problem::Http { status, message }),
                UsageReply::BadBody(e) => Err(Problem::BadReply(e)),
            },
        };
        Attempt {
            used_slot: true,
            result,
        }
    }

    /// Asks the official binary to refresh the token, then reads the file again.
    fn refresh(
        &self,
        dir: &Path,
        path: &Path,
        before: Credentials,
        busy: &Busy,
    ) -> Result<Credentials, Problem> {
        let Some(cli) = &self.cli else {
            return self.still_usable(before, "Cannot find the claude binary to refresh it.");
        };
        let first = cli.run(dir, &AUTH_STATUS_ARGS, self.auth_status_timeout, busy);
        let after = read_now(path)?;
        if after.expires_at != before.expires_at {
            return Ok(after);
        }
        // A binary that still runs can still use the refresh token. A second run at the same
        // time can spend the same refresh token twice, so the watcher waits for the next slot.
        if first == RunEnd::TimedOut {
            return self.still_usable(after, "claude auth status ran past the timeout.");
        }
        let second = cli.run(dir, &PRINT_USAGE_ARGS, self.print_usage_timeout, busy);
        let after = read_now(path)?;
        if after.expires_at != before.expires_at {
            return Ok(after);
        }
        let why = match second {
            RunEnd::TimedOut => "claude -p /usage ran past the timeout.".to_owned(),
            RunEnd::NotStarted(e) => format!("The claude binary did not start. {e}"),
            RunEnd::Exited(_) => "The claude binary did not refresh it.".to_owned(),
        };
        self.still_usable(after, &why)
    }

    /// Returns the credentials when the token is still valid, else an `Expired` problem.
    fn still_usable(&self, creds: Credentials, why: &str) -> Result<Credentials, Problem> {
        if creds.expires_at > self.clock.now() {
            Ok(creds)
        } else {
            Err(Problem::Expired(why.to_owned()))
        }
    }

    /// Returns `claude-code/<version>`.
    ///
    /// The watcher reads the version again after `VERSION_RECHECK`, so a claude update shows
    /// up within an hour. A failed read keeps the last known version.
    fn user_agent(&mut self) -> String {
        let now = self.clock.now();
        let due = self
            .version_checked
            .is_none_or(|at| now - at >= VERSION_RECHECK);
        if due {
            self.version_checked = Some(now);
            if let Some(v) = self.cli.as_ref().and_then(ClaudeCli::version) {
                self.user_agent = Some(format!("claude-code/{v}"));
            }
        }
        self.user_agent
            .clone()
            .unwrap_or_else(|| "claude-code/unknown".to_owned())
    }
}

fn read_now(path: &Path) -> Result<Credentials, Problem> {
    match read_credentials(path) {
        Ok(Some(creds)) => Ok(creds),
        Ok(None) => Err(Problem::NoCredentials),
        Err(e) => Err(Problem::BadCredentials(e.to_string())),
    }
}

/// Returns the new state of an account after an attempt that ended at `done`.
fn apply(state: AccountState, attempt: Attempt, done: DateTime<Utc>) -> AccountState {
    let interval = match &attempt.result {
        Ok(_) => CALL_INTERVAL,
        Err(Problem::RateLimited) => (state.interval * 2).min(MAX_CALL_INTERVAL),
        Err(_) => state.interval,
    };
    let next_call = if attempt.used_slot {
        Some(done + interval)
    } else {
        state.next_call
    };
    let record = match attempt.result {
        Ok(windows) => WatchRecord {
            reading: Some(Reading {
                windows,
                fetched_at: done,
            }),
            problem: None,
            ..state.record
        },
        Err(problem) => WatchRecord {
            problem: Some(problem),
            ..state.record
        },
    };
    AccountState {
        record,
        next_call,
        interval,
        busy: state.busy,
    }
}
