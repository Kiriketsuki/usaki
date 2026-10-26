//! Runs the official `claude` binary in a watcher dir. A trait keeps the binary out of the tests.
//!
//! The binary refreshes the token of a watcher login. usaKi never sends SIGKILL to it: a kill
//! between the use of the old refresh token and the write of the new one ends the login. At the
//! timeout, usaKi sends SIGTERM and lets a background thread wait for the exit.

use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// The longest time that `claude --version` may take.
pub const VERSION_TIMEOUT: Duration = Duration::from_secs(20);
/// The time between two exit checks of a running binary.
const WAIT_STEP: Duration = Duration::from_millis(200);

/// How a run of the binary ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEnd {
    /// The binary exited. The value is its exit code, if it has one.
    Exited(Option<i32>),
    /// The binary ran past the timeout. usaKi asked it to stop and did not wait.
    TimedOut,
    /// The binary did not start.
    NotStarted(String),
}

/// A flag that is set while a refresh child of one watcher dir has not exited.
///
/// Two runs in one watcher dir at the same time can spend one refresh token twice. The watcher
/// makes no refresh and no API call for an account while its flag is set. The code that reaps
/// the child clears the flag. For a child past its timeout, that is the reaper thread.
#[derive(Debug, Clone, Default)]
pub struct Busy(Arc<AtomicBool>);

impl Busy {
    pub fn is_set(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    pub fn set(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn clear(&self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// The official `claude` binary.
pub trait ClaudeCli {
    /// Returns the version, for example `2.1.310`, or `None` when it is not known.
    fn version(&self) -> Option<String>;
    /// Runs the binary with `CLAUDE_CONFIG_DIR` set to `config_dir`.
    ///
    /// The run sets `busy` and clears it when the child exits. After `RunEnd::TimedOut` the
    /// flag stays set until the reaper thread sees the exit.
    fn run(&self, config_dir: &Path, args: &[&str], timeout: Duration, busy: &Busy) -> RunEnd;
}

/// Returns the file name of the binary on this platform.
fn binary_name() -> String {
    format!("claude{}", std::env::consts::EXE_SUFFIX)
}

/// Finds the binary: each dir of `path_var` first, then `<home>/.local/bin`.
pub fn find_binary(path_var: Option<&OsStr>, home: Option<&Path>) -> Option<PathBuf> {
    let name = binary_name();
    let on_path = path_var
        .into_iter()
        .flat_map(std::env::split_paths)
        .map(|dir| dir.join(&name));
    let local = home.map(|h| h.join(".local").join("bin").join(&name));
    on_path.chain(local).find(|p| is_executable(p))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

/// Returns the first word of the `claude --version` output when it looks like a version.
pub fn parse_version(stdout: &str) -> Option<String> {
    let word = stdout.split_whitespace().next()?;
    let safe = word.len() <= 40
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'));
    safe.then(|| word.to_owned())
}

/// The real binary.
#[derive(Debug, Clone)]
pub struct ClaudeBinary {
    path: PathBuf,
}

impl ClaudeBinary {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Finds the binary from the process environment.
    pub fn find() -> Option<Self> {
        let path_var = std::env::var_os("PATH");
        find_binary(path_var.as_deref(), dirs::home_dir().as_deref()).map(Self::new)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl ClaudeCli for ClaudeBinary {
    fn version(&self) -> Option<String> {
        let child = Command::new(&self.path)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let (_, mut child) = wait_or_release(child, VERSION_TIMEOUT, Busy::default())?;
        let mut out = String::new();
        child.stdout.take()?.read_to_string(&mut out).ok()?;
        parse_version(&out)
    }

    fn run(&self, config_dir: &Path, args: &[&str], timeout: Duration, busy: &Busy) -> RunEnd {
        busy.set();
        let spawned = Command::new(&self.path)
            .args(args)
            .env("CLAUDE_CONFIG_DIR", config_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        let child = match spawned {
            Ok(child) => child,
            Err(e) => {
                busy.clear();
                return RunEnd::NotStarted(e.to_string());
            }
        };
        match wait_or_release(child, timeout, busy.clone()) {
            Some((status, _)) => RunEnd::Exited(status.code()),
            None => RunEnd::TimedOut,
        }
    }
}

/// Waits for the child until the timeout and returns the exit status with the child.
///
/// At the timeout it asks the child to stop, hands the child to a thread that waits for the
/// exit, and returns `None`. The thread reaps the child, so no zombie stays, and then clears
/// `busy`. A child that ignores SIGTERM keeps `busy` set for as long as it runs.
fn wait_or_release(mut child: Child, timeout: Duration, busy: Busy) -> Option<(ExitStatus, Child)> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                busy.clear();
                return Some((status, child));
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(WAIT_STEP),
            Ok(None) | Err(_) => break,
        }
    }
    ask_to_stop(&child);
    std::thread::spawn(move || {
        let _ = child.wait();
        busy.clear();
    });
    None
}

#[cfg(unix)]
fn ask_to_stop(child: &Child) {
    if let Ok(pid) = libc::pid_t::try_from(child.id()) {
        // SAFETY: kill only sends a signal. The child is not reaped yet, so the pid still
        // names it. SIGTERM lets the binary finish a token write before it exits.
        unsafe {
            libc::kill(pid, libc::SIGTERM);
        }
    }
}

#[cfg(not(unix))]
fn ask_to_stop(_child: &Child) {
    // Windows has no SIGTERM. TerminateProcess acts like SIGKILL, so usaKi lets the binary run.
}
