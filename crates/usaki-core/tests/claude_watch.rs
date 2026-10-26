//! Tests for the Claude watcher provider. All tokens and fixture data are synthetic.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use chrono::{DateTime, Duration, TimeZone, Utc};
use usaki_core::providers::claude_watch::cli::{find_binary, parse_version};
use usaki_core::providers::claude_watch::usage::error_message;
use usaki_core::providers::claude_watch::{
    AccessToken, Busy, CALL_INTERVAL, ClaudeCli, HttpReply, MAX_CALL_INTERVAL, Problem, RunEnd,
    UsageClient, UsageReply, Watcher, classify_reply, credentials_path, discover_accounts,
    parse_credentials, parse_usage_body, read_credentials, watch_root,
};
use usaki_core::{Clock, Error, Source, WindowKind};

const START: i64 = 1_800_000_000;
const TOKEN: &str = "synthetic-token-acc1";
const NEW_TOKEN: &str = "synthetic-token-acc1-new";

fn at(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(secs, 0).unwrap()
}

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/watch")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

// ---- Usage reply ----

#[test]
fn normal_reply_gives_both_windows() {
    let windows = parse_usage_body(&fixture("usage_normal.json")).unwrap();
    assert_eq!(windows.len(), 2);
    assert_eq!(windows[0].kind, WindowKind::FiveHour);
    assert_eq!(windows[0].used_pct, 42.5);
    assert_eq!(
        windows[0].resets_at.map(|t| t.timestamp()),
        Some(
            Utc.with_ymd_and_hms(2027, 1, 15, 8, 0, 0)
                .unwrap()
                .timestamp()
        )
    );
    assert_eq!(windows[1].kind, WindowKind::SevenDay);
    assert_eq!(windows[1].used_pct, 17.0);
}

#[test]
fn null_windows_give_no_windows() {
    let windows = parse_usage_body(&fixture("usage_null_windows.json")).unwrap();
    assert!(windows.is_empty());
}

#[test]
fn extra_keys_are_ignored() {
    let windows = parse_usage_body(&fixture("usage_extra_keys.json")).unwrap();
    let kinds: Vec<WindowKind> = windows.iter().map(|w| w.kind).collect();
    assert_eq!(kinds, [WindowKind::FiveHour, WindowKind::SevenDay]);
    assert_eq!(windows[0].used_pct, 0.0);
    assert_eq!(windows[0].resets_at, None);
    assert_eq!(windows[1].used_pct, 100.0);
}

#[test]
fn error_body_gives_its_message() {
    let body = fixture("usage_error.json");
    assert_eq!(
        error_message(&body).as_deref(),
        Some("Rate limited. Please try again later.")
    );
    assert_eq!(
        classify_reply(429, &body),
        UsageReply::RateLimited(Some("Rate limited. Please try again later.".into()))
    );
    assert!(matches!(
        classify_reply(401, &body),
        UsageReply::Unauthorized(Some(_))
    ));
}

#[test]
fn other_status_and_bad_bodies_classify() {
    assert_eq!(
        classify_reply(500, "<html>oops</html>"),
        UsageReply::Failed {
            status: 500,
            message: None
        }
    );
    assert!(matches!(
        classify_reply(200, "{ not json"),
        UsageReply::BadBody(_)
    ));
    assert!(matches!(
        classify_reply(200, &fixture("usage_error.json")),
        UsageReply::Usage(w) if w.is_empty()
    ));
}

#[test]
fn bad_window_values_are_errors() {
    assert!(parse_usage_body(r#"{"five_hour":{"utilization":-1}}"#).is_err());
    assert!(parse_usage_body(r#"{"five_hour":{"utilization":"10"}}"#).is_err());
    assert!(parse_usage_body(r#"{"five_hour":{"utilization":1,"resets_at":1800000000}}"#).is_err());
    assert!(parse_usage_body(r#"{"five_hour":3}"#).is_err());
    assert!(parse_usage_body("[]").is_err());
}

#[test]
fn long_error_message_is_cut() {
    let body = format!(r#"{{"error":{{"message":"{}"}}}}"#, "x".repeat(500));
    assert_eq!(error_message(&body).unwrap().len(), 200);
    assert_eq!(
        error_message(r#"{"error":{"type":"authentication_error"}}"#).as_deref(),
        Some("authentication_error")
    );
}

// ---- Credentials ----

fn creds_json(token: &str, expires_ms: i64) -> String {
    format!(
        r#"{{"claudeAiOauth":{{"accessToken":"{token}","refreshToken":"synthetic-refresh",
            "expiresAt":{expires_ms},"scopes":["user:inference"]}}}}"#
    )
}

fn write_creds(dir: &Path, token: &str, expires_at: DateTime<Utc>) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        credentials_path(dir),
        creds_json(token, expires_at.timestamp_millis()),
    )
    .unwrap();
}

#[test]
fn credentials_parse() {
    let c = parse_credentials(&creds_json(TOKEN, 1_800_000_123_456), Path::new("x")).unwrap();
    assert_eq!(c.token.expose(), TOKEN);
    assert_eq!(c.expires_at.timestamp_millis(), 1_800_000_123_456);
}

#[test]
fn credentials_errors_never_hold_the_token() {
    let cases = [
        format!(r#"{{"claudeAiOauth":{{"accessToken":"{TOKEN}"}}}}"#),
        format!(r#"{{"claudeAiOauth":{{"accessToken":"{TOKEN}","expiresAt":"soon"}}}}"#),
        format!(r#"{{"claudeAiOauth":{{"accessToken":"{TOKEN}","expiresAt":1,}}}}"#),
        r#"{"claudeAiOauth":{"accessToken":"","expiresAt":1}}"#.to_owned(),
        r#"{"other":{}}"#.to_owned(),
    ];
    for text in cases {
        let err = parse_credentials(&text, Path::new("x")).unwrap_err();
        assert!(
            matches!(err, Error::InvalidField { .. } | Error::Malformed { .. }),
            "{err}"
        );
        assert!(!err.to_string().contains(TOKEN));
    }
}

#[test]
fn token_debug_is_hidden() {
    let token = AccessToken::new(TOKEN);
    assert!(!format!("{token:?}").contains(TOKEN));
}

#[test]
fn missing_credentials_file_gives_none() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        read_credentials(&credentials_path(dir.path()))
            .unwrap()
            .is_none()
    );
}

// ---- Discovery and binary ----

#[test]
fn discovers_valid_subdirs_sorted() {
    let data = tempfile::tempdir().unwrap();
    let root = watch_root(data.path());
    assert!(discover_accounts(&root).unwrap().is_empty());
    for name in ["acc2", "acc1", "a.b", "has space"] {
        std::fs::create_dir_all(root.join(name)).unwrap();
    }
    std::fs::write(root.join("acc3"), "").unwrap();
    assert_eq!(discover_accounts(&root).unwrap(), ["acc1", "acc2"]);
    assert_eq!(root, data.path().join("usaki").join("watch"));
}

#[cfg(unix)]
fn make_exec(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "#!/bin/sh\n").unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(unix)]
#[test]
fn finds_binary_on_path_then_local_bin() {
    let tmp = tempfile::tempdir().unwrap();
    let on_path = tmp.path().join("bin");
    let home = tmp.path().join("home");
    let local = home.join(".local/bin/claude");
    make_exec(&local);
    std::fs::create_dir_all(&on_path).unwrap();
    let path_var = std::env::join_paths([on_path.clone()]).unwrap();
    assert_eq!(find_binary(Some(&path_var), Some(&home)), Some(local));
    make_exec(&on_path.join("claude"));
    assert_eq!(
        find_binary(Some(&path_var), Some(&home)),
        Some(on_path.join("claude"))
    );
    assert_eq!(find_binary(None, None), None);
}

#[test]
fn version_comes_from_the_first_word() {
    assert_eq!(
        parse_version("2.1.310 (Claude Code)\n").as_deref(),
        Some("2.1.310")
    );
    assert_eq!(parse_version(""), None);
    assert_eq!(parse_version("bad\r\nheader"), Some("bad".into()));
    assert_eq!(parse_version("a:b"), None);
}

#[cfg(unix)]
fn script(dir: &Path, body: &str) -> usaki_core::providers::claude_watch::ClaudeBinary {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("claude");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    usaki_core::providers::claude_watch::ClaudeBinary::new(path)
}

#[cfg(unix)]
#[test]
fn binary_runs_in_the_watcher_dir_and_gives_its_version() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = script(
        tmp.path(),
        r#"[ "$1" = --version ] && { echo "1.2.3 (Claude Code)"; exit 0; }
echo "$*" > "$CLAUDE_CONFIG_DIR/args""#,
    );
    assert_eq!(bin.version().as_deref(), Some("1.2.3"));
    let watch = tmp.path().join("acc1");
    std::fs::create_dir_all(&watch).unwrap();
    let busy = Busy::default();
    let end = bin.run(
        &watch,
        &["auth", "status", "--json"],
        std::time::Duration::from_secs(10),
        &busy,
    );
    assert_eq!(end, RunEnd::Exited(Some(0)));
    assert!(!busy.is_set(), "an exited child clears the flag");
    let args = std::fs::read_to_string(watch.join("args")).unwrap();
    assert_eq!(args.trim(), "auth status --json");
}

#[cfg(unix)]
#[test]
fn binary_past_the_timeout_gets_sigterm_not_sigkill() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = script(
        tmp.path(),
        r#"trap 'echo term > "$CLAUDE_CONFIG_DIR/term"; exit 0' TERM
sleep 10 &
wait"#,
    );
    let end = bin.run(
        tmp.path(),
        &["-p", "/usage"],
        std::time::Duration::from_millis(300),
        &Busy::default(),
    );
    assert_eq!(end, RunEnd::TimedOut);
    // The trap runs only on SIGTERM. A shell cannot trap SIGKILL.
    let term = tmp.path().join("term");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !term.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(term.exists(), "the binary did not get SIGTERM");
}

#[cfg(unix)]
fn wait_until(what: impl Fn() -> bool, limit: std::time::Duration) -> bool {
    let deadline = std::time::Instant::now() + limit;
    while !what() {
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    true
}

/// A stand-in for a binary that ignores SIGTERM. It runs for about 2 s.
#[cfg(unix)]
const IGNORES_TERM: &str = "trap '' TERM\nsleep 2";

#[cfg(unix)]
#[test]
fn child_that_ignores_sigterm_stays_busy_until_it_exits() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = script(tmp.path(), IGNORES_TERM);
    let busy = Busy::default();
    let end = bin.run(
        tmp.path(),
        &["auth", "status", "--json"],
        std::time::Duration::from_millis(200),
        &busy,
    );
    assert_eq!(end, RunEnd::TimedOut);
    std::thread::sleep(std::time::Duration::from_millis(800));
    assert!(busy.is_set(), "the child ignored SIGTERM and still runs");
    assert!(
        wait_until(|| !busy.is_set(), std::time::Duration::from_secs(10)),
        "the reaper did not clear the flag after the exit"
    );
}

// ---- Watcher ----

#[derive(Clone)]
struct TestClock(Rc<Cell<DateTime<Utc>>>);

impl TestClock {
    fn advance(&self, by: Duration) {
        self.0.set(self.0.get() + by);
    }
}

impl Clock for TestClock {
    fn now(&self) -> DateTime<Utc> {
        self.0.get()
    }
}

/// A fake API. It pops one reply per call and records the token and user agent.
#[derive(Clone, Default)]
struct FakeHttp {
    replies: Rc<RefCell<Vec<Result<HttpReply, String>>>>,
    calls: Rc<RefCell<Vec<(String, String)>>>,
}

impl FakeHttp {
    fn push(&self, status: u16, body: &str) {
        self.replies.borrow_mut().push(Ok(HttpReply {
            status,
            body: body.to_owned(),
        }));
    }

    fn calls(&self) -> usize {
        self.calls.borrow().len()
    }
}

impl UsageClient for FakeHttp {
    fn fetch_usage(&self, token: &AccessToken, user_agent: &str) -> Result<HttpReply, String> {
        self.calls
            .borrow_mut()
            .push((token.expose().to_owned(), user_agent.to_owned()));
        let mut replies = self.replies.borrow_mut();
        if replies.is_empty() {
            return Err("no reply queued".into());
        }
        replies.remove(0)
    }
}

type Action = Rc<dyn Fn(&Path, &[&str]) -> RunEnd>;

/// A fake binary. It records each run and calls `action`, which may rewrite the credentials.
///
/// A run that ends in `TimedOut` keeps its busy flag set, like a child that ignores SIGTERM.
/// `exit_all` plays the reaper and clears those flags.
#[derive(Clone)]
struct FakeCli {
    runs: Rc<RefCell<Vec<String>>>,
    action: Action,
    held: Rc<RefCell<Vec<Busy>>>,
    version: Rc<RefCell<String>>,
    version_reads: Rc<Cell<u32>>,
}

impl FakeCli {
    fn new(action: impl Fn(&Path, &[&str]) -> RunEnd + 'static) -> Self {
        Self {
            runs: Rc::default(),
            action: Rc::new(action),
            held: Rc::default(),
            version: Rc::new(RefCell::new("9.9.9".into())),
            version_reads: Rc::default(),
        }
    }

    fn runs(&self) -> Vec<String> {
        self.runs.borrow().clone()
    }

    fn exit_all(&self) {
        self.held.borrow_mut().drain(..).for_each(|b| b.clear());
    }
}

impl ClaudeCli for FakeCli {
    fn version(&self) -> Option<String> {
        self.version_reads.set(self.version_reads.get() + 1);
        Some(self.version.borrow().clone())
    }

    fn run(
        &self,
        config_dir: &Path,
        args: &[&str],
        _timeout: std::time::Duration,
        busy: &Busy,
    ) -> RunEnd {
        self.runs.borrow_mut().push(args.join(" "));
        busy.set();
        let end = (self.action)(config_dir, args);
        if end == RunEnd::TimedOut {
            self.held.borrow_mut().push(busy.clone());
        } else {
            busy.clear();
        }
        end
    }
}

struct Setup {
    _data: tempfile::TempDir,
    root: PathBuf,
    clock: TestClock,
    http: FakeHttp,
}

impl Setup {
    fn new() -> Self {
        let data = tempfile::tempdir().unwrap();
        let root = watch_root(data.path());
        Self {
            _data: data,
            root,
            clock: TestClock(Rc::new(Cell::new(at(START)))),
            http: FakeHttp::default(),
        }
    }

    fn dir(&self, id: &str) -> PathBuf {
        self.root.join(id)
    }

    fn login(&self, id: &str, expires_in: Duration) {
        write_creds(&self.dir(id), TOKEN, self.clock.now() + expires_in);
    }

    fn watcher(&self, cli: Option<FakeCli>) -> Watcher<TestClock, FakeHttp, FakeCli> {
        Watcher::new(
            self.root.clone(),
            self.clock.clone(),
            self.http.clone(),
            cli,
        )
    }
}

fn idle_cli() -> FakeCli {
    FakeCli::new(|_, _| RunEnd::Exited(Some(0)))
}

fn ok_body() -> String {
    fixture("usage_normal.json")
}

#[test]
fn good_reply_gives_a_fresh_snapshot() {
    let s = Setup::new();
    s.login("acc1", Duration::hours(8));
    s.http.push(200, &ok_body());
    let mut w = s.watcher(Some(idle_cli()));
    let records = w.tick(|_| false).unwrap();
    assert_eq!(records.len(), 1);
    let snap = records[0].snapshot(s.clock.now());
    assert_eq!(snap.account_id, "acc1");
    assert_eq!(snap.source, Source::ClaudeWatch);
    assert!(!snap.stale);
    assert_eq!(snap.stale_reason, None);
    assert_eq!(snap.window(WindowKind::FiveHour).unwrap().used_pct, 42.5);
    let calls = s.http.calls.borrow();
    assert_eq!(calls[0], (TOKEN.to_owned(), "claude-code/9.9.9".to_owned()));
}

#[test]
fn calls_at_most_once_per_interval() {
    let s = Setup::new();
    s.login("acc1", Duration::hours(8));
    let mut w = s.watcher(Some(idle_cli()));
    for _ in 0..3 {
        s.http.push(200, &ok_body());
    }
    w.tick(|_| false).unwrap();
    for _ in 0..9 {
        s.clock.advance(Duration::seconds(30));
        w.tick(|_| false).unwrap();
    }
    assert_eq!(s.http.calls(), 1, "4.5 minutes after the first call");
    s.clock.advance(Duration::seconds(30));
    w.tick(|_| false).unwrap();
    assert_eq!(s.http.calls(), 2, "5 minutes after the first call");
}

#[test]
fn rate_limit_doubles_the_interval_up_to_the_cap() {
    let s = Setup::new();
    s.login("acc1", Duration::days(30));
    let mut w = s.watcher(Some(idle_cli()));
    let limited = fixture("usage_error.json");
    let mut gaps = Vec::new();
    for _ in 0..6 {
        s.http.push(429, &limited);
        let before = s.http.calls();
        let mut waited = Duration::zero();
        w.tick(|_| false).unwrap();
        while s.http.calls() == before {
            s.clock.advance(Duration::minutes(1));
            waited += Duration::minutes(1);
            w.tick(|_| false).unwrap();
        }
        gaps.push(waited.num_minutes());
    }
    // The first call is due at once. Each 429 then doubles the wait, up to 60 minutes.
    assert_eq!(gaps, [0, 10, 20, 40, 60, 60]);
    assert_eq!(MAX_CALL_INTERVAL, Duration::minutes(60));

    s.http.push(200, &ok_body());
    s.clock.advance(Duration::minutes(60));
    w.tick(|_| false).unwrap();
    s.http.push(200, &ok_body());
    s.clock.advance(CALL_INTERVAL);
    let records = w.tick(|_| false).unwrap();
    assert_eq!(
        s.http.calls(),
        8,
        "a good reply resets the interval to 5 minutes"
    );
    assert!(!records[0].snapshot(s.clock.now()).stale);
}

#[test]
fn rate_limit_keeps_a_recent_reading_fresh() {
    let s = Setup::new();
    s.login("acc1", Duration::days(1));
    let mut w = s.watcher(Some(idle_cli()));
    s.http.push(200, &ok_body());
    w.tick(|_| false).unwrap();
    s.http.push(429, "");
    s.clock.advance(CALL_INTERVAL);
    let records = w.tick(|_| false).unwrap();
    assert_eq!(records[0].problem, Some(Problem::RateLimited));
    assert!(!records[0].snapshot(s.clock.now()).stale);
    let later = records[0].snapshot(s.clock.now() + Duration::minutes(11));
    assert!(later.stale);
    assert!(later.stale_reason.unwrap().contains("429"));
}

#[test]
fn unauthorized_marks_the_account_stale() {
    let s = Setup::new();
    s.login("acc1", Duration::hours(8));
    let mut w = s.watcher(Some(idle_cli()));
    s.http.push(200, &ok_body());
    w.tick(|_| false).unwrap();
    s.http
        .push(401, r#"{"error":{"message":"Invalid token."}}"#);
    s.clock.advance(CALL_INTERVAL);
    let snap = w.tick(|_| false).unwrap()[0].snapshot(s.clock.now());
    assert!(snap.stale);
    assert_eq!(snap.windows.len(), 2, "the old windows stay for display");
    let reason = snap.stale_reason.unwrap();
    assert!(reason.contains("401") && reason.contains("Invalid token."));
}

#[test]
fn transport_failure_and_age_make_the_account_stale() {
    let s = Setup::new();
    s.login("acc1", Duration::days(1));
    let mut w = s.watcher(Some(idle_cli()));
    s.http.push(200, &ok_body());
    w.tick(|_| false).unwrap();
    s.clock.advance(CALL_INTERVAL);
    let records = w.tick(|_| false).unwrap();
    assert_eq!(
        records[0].problem,
        Some(Problem::Transport("no reply queued".into()))
    );
    assert!(!records[0].snapshot(s.clock.now()).stale);
    assert!(
        records[0]
            .snapshot(s.clock.now() + Duration::minutes(11))
            .stale
    );
}

#[test]
fn auth_status_refreshes_an_expiring_token() {
    let s = Setup::new();
    s.login("acc1", Duration::seconds(90));
    let new_expiry = s.clock.now() + Duration::hours(8);
    let cli = FakeCli::new(move |dir, args| {
        if args[0] == "auth" {
            write_creds(dir, NEW_TOKEN, new_expiry);
        }
        RunEnd::Exited(Some(0))
    });
    s.http.push(200, &ok_body());
    let mut w = s.watcher(Some(cli.clone()));
    let records = w.tick(|_| false).unwrap();
    assert_eq!(cli.runs(), ["auth status --json"]);
    assert_eq!(s.http.calls.borrow()[0].0, NEW_TOKEN);
    assert!(!records[0].snapshot(s.clock.now()).stale);
}

#[test]
fn print_usage_runs_when_auth_status_does_not_refresh() {
    let s = Setup::new();
    s.login("acc1", -Duration::minutes(10));
    let new_expiry = s.clock.now() + Duration::hours(8);
    let cli = FakeCli::new(move |dir, args| {
        if args[0] == "-p" {
            write_creds(dir, NEW_TOKEN, new_expiry);
        }
        RunEnd::Exited(Some(0))
    });
    s.http.push(200, &ok_body());
    let mut w = s.watcher(Some(cli.clone()));
    w.tick(|_| false).unwrap();
    assert_eq!(cli.runs(), ["auth status --json", "-p /usage"]);
    assert_eq!(s.http.calls.borrow()[0].0, NEW_TOKEN);
}

#[test]
fn expired_token_without_refresh_makes_no_call() {
    let s = Setup::new();
    s.login("acc1", -Duration::minutes(1));
    let cli = idle_cli();
    let mut w = s.watcher(Some(cli.clone()));
    let snap = w.tick(|_| false).unwrap()[0].snapshot(s.clock.now());
    assert_eq!(s.http.calls(), 0);
    assert!(snap.stale);
    assert!(snap.stale_reason.unwrap().contains("expired"));
    // The refresh used the call slot, so the binary does not run again at once.
    s.clock.advance(Duration::minutes(1));
    w.tick(|_| false).unwrap();
    assert_eq!(cli.runs().len(), 2);
    s.clock.advance(CALL_INTERVAL);
    w.tick(|_| false).unwrap();
    assert_eq!(cli.runs().len(), 4);
}

#[test]
fn valid_token_is_used_when_the_refresh_does_not_change_it() {
    let s = Setup::new();
    s.login("acc1", Duration::seconds(60));
    let cli = idle_cli();
    s.http.push(200, &ok_body());
    let mut w = s.watcher(Some(cli.clone()));
    w.tick(|_| false).unwrap();
    assert_eq!(cli.runs().len(), 2);
    assert_eq!(s.http.calls(), 1);
}

#[test]
fn timed_out_auth_status_stops_the_refresh() {
    let s = Setup::new();
    s.login("acc1", -Duration::minutes(1));
    let cli = FakeCli::new(|_, _| RunEnd::TimedOut);
    let mut w = s.watcher(Some(cli.clone()));
    let records = w.tick(|_| false).unwrap();
    assert_eq!(cli.runs(), ["auth status --json"]);
    assert!(matches!(records[0].problem, Some(Problem::Expired(_))));
}

#[test]
fn busy_account_makes_no_refresh_and_no_call_until_the_child_exits() {
    let s = Setup::new();
    s.login("acc1", Duration::seconds(60));
    let new_expiry = s.clock.now() + Duration::hours(8);
    let first = Rc::new(Cell::new(true));
    let cli = FakeCli::new({
        let first = first.clone();
        move |dir, _| {
            if first.replace(false) {
                return RunEnd::TimedOut;
            }
            write_creds(dir, NEW_TOKEN, new_expiry);
            RunEnd::Exited(Some(0))
        }
    });
    let mut w = s.watcher(Some(cli.clone()));
    let records = w.tick(|_| false).unwrap();
    assert_eq!(records[0].problem, Some(Problem::RefreshRunning));
    assert!(w.is_busy("acc1"));
    assert_eq!(
        s.http.calls(),
        0,
        "the token is valid, but a refresh child still runs"
    );

    // The child ignores SIGTERM and runs on, across several call slots.
    for _ in 0..3 {
        s.clock.advance(CALL_INTERVAL);
        w.tick(|_| false).unwrap();
    }
    assert_eq!(cli.runs(), ["auth status --json"]);
    assert_eq!(s.http.calls(), 0);

    // The reaper sees the exit and clears the flag. The next tick refreshes and calls.
    cli.exit_all();
    assert!(!w.is_busy("acc1"));
    s.http.push(200, &ok_body());
    let records = w.tick(|_| false).unwrap();
    assert_eq!(cli.runs(), ["auth status --json", "auth status --json"]);
    assert_eq!(s.http.calls.borrow()[0].0, NEW_TOKEN);
    assert!(!records[0].snapshot(s.clock.now()).stale);
}

#[cfg(unix)]
#[test]
fn real_child_that_ignores_sigterm_blocks_later_slots() {
    use usaki_core::providers::claude_watch::ClaudeBinary;
    let s = Setup::new();
    s.login("acc1", -Duration::minutes(1));
    let tmp = tempfile::tempdir().unwrap();
    let log = tmp.path().join("runs.log");
    let bin = script(
        tmp.path(),
        &format!("echo \"$*\" >> '{}'\n{IGNORES_TERM}", log.display()),
    );
    let short = std::time::Duration::from_millis(200);
    let mut w: Watcher<TestClock, FakeHttp, ClaudeBinary> =
        Watcher::new(s.root.clone(), s.clock.clone(), s.http.clone(), Some(bin))
            .with_timeouts(short, short);
    let runs = || {
        std::fs::read_to_string(&log)
            .map(|t| t.lines().count())
            .unwrap_or(0)
    };

    w.tick(|_| false).unwrap();
    assert_eq!(
        runs(),
        1,
        "auth status timed out, so -p /usage did not start"
    );
    assert!(w.is_busy("acc1"));
    s.clock.advance(CALL_INTERVAL);
    w.tick(|_| false).unwrap();
    assert_eq!(runs(), 1, "the busy account skips its next slot");

    assert!(wait_until(
        || !w.is_busy("acc1"),
        std::time::Duration::from_secs(10)
    ));
    w.tick(|_| false).unwrap();
    assert_eq!(runs(), 2, "the slot after the exit refreshes again");
    assert_eq!(s.http.calls(), 0, "the token stays expired, so no call");
    assert!(wait_until(
        || !w.is_busy("acc1"),
        std::time::Duration::from_secs(10)
    ));
}

#[test]
fn version_is_read_again_each_hour() {
    let s = Setup::new();
    s.login("acc1", Duration::days(1));
    let cli = idle_cli();
    let mut w = s.watcher(Some(cli.clone()));
    let agents = || -> Vec<String> { s.http.calls.borrow().iter().map(|c| c.1.clone()).collect() };
    for step in 0..24 {
        if step == 1 {
            *cli.version.borrow_mut() = "9.9.10".into();
        }
        s.http.push(200, &ok_body());
        w.tick(|_| false).unwrap();
        s.clock.advance(CALL_INTERVAL);
    }
    assert_eq!(
        cli.version_reads.get(),
        2,
        "one read at start, one after an hour"
    );
    let agents = agents();
    assert_eq!(
        agents[11], "claude-code/9.9.9",
        "55 minutes after the first read"
    );
    assert_eq!(
        agents[12], "claude-code/9.9.10",
        "60 minutes after the first read"
    );
}

#[test]
fn no_binary_means_no_refresh() {
    let s = Setup::new();
    s.login("acc1", -Duration::minutes(1));
    let mut w = s.watcher(None);
    let records = w.tick(|_| false).unwrap();
    assert_eq!(s.http.calls(), 0);
    assert!(matches!(records[0].problem, Some(Problem::Expired(_))));
}

#[test]
fn skipped_and_unready_accounts_make_no_call() {
    let s = Setup::new();
    s.login("acc1", Duration::hours(8));
    std::fs::create_dir_all(s.dir("acc2")).unwrap();
    let mut w = s.watcher(Some(idle_cli()));
    let records = w.tick(|id| id == "acc1").unwrap();
    assert_eq!(s.http.calls(), 0);
    let ids: Vec<&str> = records.iter().map(|r| r.account_id.as_str()).collect();
    assert_eq!(ids, ["acc1", "acc2"]);
    assert_eq!(records[0].problem, None);
    assert_eq!(records[1].problem, Some(Problem::NoCredentials));
    let snap = records[1].snapshot(s.clock.now());
    assert!(snap.stale && snap.stale_reason.unwrap().contains("claude auth login"));

    // A login made later shows up on the next tick, without a wait for a call slot.
    s.login("acc2", Duration::hours(8));
    s.http.push(200, &ok_body());
    s.clock.advance(Duration::seconds(30));
    w.tick(|id| id == "acc1").unwrap();
    assert_eq!(s.http.calls(), 1);
}

#[test]
fn removed_login_drops_its_record() {
    let s = Setup::new();
    s.login("acc1", Duration::hours(8));
    s.login("acc2", Duration::hours(8));
    let mut w = s.watcher(Some(idle_cli()));
    assert_eq!(w.tick(|_| true).unwrap().len(), 2);
    std::fs::remove_dir_all(s.dir("acc2")).unwrap();
    assert_eq!(w.tick(|_| true).unwrap().len(), 1);
}

#[test]
fn bad_credentials_give_a_reason_without_the_token() {
    let s = Setup::new();
    std::fs::create_dir_all(s.dir("acc1")).unwrap();
    std::fs::write(
        credentials_path(&s.dir("acc1")),
        format!(r#"{{"claudeAiOauth":{{"accessToken":"{TOKEN}"}}}}"#),
    )
    .unwrap();
    let mut w = s.watcher(Some(idle_cli()));
    let snap = w.tick(|_| false).unwrap()[0].snapshot(s.clock.now());
    let reason = snap.stale_reason.unwrap();
    assert!(reason.contains("expiresAt"));
    assert!(!reason.contains(TOKEN));
}

// ---- Guard ----

/// usaKi never writes a credentials file and never refreshes a token itself. This scan fails
/// when the watcher module gains any file write or any use of the refresh token.
#[test]
fn watcher_module_never_writes_files_or_refreshes() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/providers/claude_watch");
    let forbidden = [
        "File::create",
        "OpenOptions",
        "fs::write",
        "fs::rename",
        "fs::copy",
        "remove_file",
        "set_permissions",
        "create_dir",
        "write_all",
        ".write(true)",
        ".append(true)",
        "refreshToken",
        "refresh_token",
        "oauth/token",
    ];
    let mut scanned = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).unwrap();
            for needle in forbidden {
                assert!(
                    !text.contains(needle),
                    "{} holds the forbidden text {needle}",
                    path.display()
                );
            }
            scanned += 1;
        }
    }
    assert!(scanned >= 6, "the scan found only {scanned} files");
}
