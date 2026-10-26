//! Tests for the Codex logs provider. Fixture data is synthetic.

use std::fs::{File, FileTimes};
use std::path::{Path, PathBuf};
use std::time::{Duration as StdDuration, SystemTime};

use chrono::{DateTime, TimeZone, Utc};
use usaki_core::providers::codex_logs::{
    CodexLogsProvider, ScanCache, codex_home_from, last_event, newest_event, parse_line,
    sessions_dir,
};
use usaki_core::{Account, Clock, Credits, Error, Provider, ProviderKind, Source, WindowKind};

/// The time of the last event in each fixture.
const EVENT_AT: i64 = 1_799_999_000;
const FIVE_RESET: i64 = 1_800_003_600;
const SEVEN_RESET: i64 = 1_800_300_000;

#[derive(Clone, Copy)]
struct FixedClock(DateTime<Utc>);

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

fn at(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(secs, 0).unwrap()
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/codex")
        .join(name)
}

fn text(name: &str) -> String {
    std::fs::read_to_string(fixture(name)).unwrap()
}

/// Writes `body` to `<home>/sessions/<rel>` with the modified time `mtime` (epoch seconds).
fn put(home: &Path, rel: &str, body: &str, mtime: i64) -> PathBuf {
    let path = sessions_dir(home).join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, body).unwrap();
    let when = SystemTime::UNIX_EPOCH + StdDuration::from_secs(mtime as u64);
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(FileTimes::new().set_modified(when))
        .unwrap();
    path
}

fn account(home: &Path) -> Account {
    Account::new("codex", ProviderKind::CodexLogs, Some(home.to_path_buf()))
}

fn fetch_at(home: &Path, now: i64) -> usaki_core::Result<usaki_core::Snapshot> {
    CodexLogsProvider::new(FixedClock(at(now))).fetch(&account(home))
}

fn home_with(name: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    put(
        dir.path(),
        &format!("2027/01/15/rollout-{name}"),
        &text(name),
        EVENT_AT,
    );
    dir
}

#[test]
fn week_only_event_gives_one_seven_day_window() {
    let home = home_with("week_only.jsonl");
    let snap = fetch_at(home.path(), EVENT_AT + 60).unwrap();
    assert_eq!(snap.account_id, "codex");
    assert_eq!(snap.source, Source::CodexLogs);
    assert!(!snap.stale);
    assert_eq!(snap.fetched_at, at(EVENT_AT));
    assert_eq!(snap.windows.len(), 1);
    let seven = snap.window(WindowKind::SevenDay).unwrap();
    assert_eq!(seven.used_pct, 37.0);
    assert_eq!(seven.resets_at, Some(at(SEVEN_RESET)));
    assert!(snap.window(WindowKind::FiveHour).is_none());
    assert_eq!(
        snap.credits,
        Some(Credits {
            has_credits: true,
            unlimited: false,
            balance: Some("123.4500".into()),
        })
    );
}

#[test]
fn windows_map_by_minutes_in_either_slot() {
    let home = home_with("both_windows.jsonl");
    let snap = fetch_at(home.path(), EVENT_AT + 60).unwrap();
    let five = snap.window(WindowKind::FiveHour).unwrap();
    assert_eq!(five.used_pct, 14.5);
    assert_eq!(five.resets_at, Some(at(FIVE_RESET)));
    assert_eq!(snap.window(WindowKind::SevenDay).unwrap().used_pct, 31.0);
}

#[test]
fn other_limit_id_is_skipped() {
    let home = home_with("other_limit.jsonl");
    let snap = fetch_at(home.path(), EVENT_AT + 60).unwrap();
    assert_eq!(snap.windows.len(), 1);
    assert_eq!(snap.window(WindowKind::SevenDay).unwrap().used_pct, 20.0);
}

#[test]
fn null_rate_limits_are_skipped() {
    let home = home_with("null_limits.jsonl");
    let snap = fetch_at(home.path(), EVENT_AT + 60).unwrap();
    assert_eq!(snap.window(WindowKind::SevenDay).unwrap().used_pct, 25.0);
}

#[test]
fn malformed_and_bad_lines_are_skipped() {
    // The file ends in a half-written line, then holds a duplicate window, a negative
    // percent and a broken line, before the good event.
    let home = home_with("malformed.jsonl");
    let snap = fetch_at(home.path(), EVENT_AT + 60).unwrap();
    assert_eq!(snap.window(WindowKind::SevenDay).unwrap().used_pct, 40.0);
}

#[test]
fn old_event_with_an_open_window_stays_fresh() {
    let home = home_with("week_only.jsonl");
    let snap = fetch_at(home.path(), EVENT_AT + 2 * 86_400).unwrap();
    assert!(!snap.stale);
    assert_eq!(snap.stale_reason, None);
    assert_eq!(snap.window(WindowKind::SevenDay).unwrap().used_pct, 37.0);
}

#[test]
fn event_past_its_reset_reports_zero_and_goes_stale() {
    let home = home_with("week_only.jsonl");
    let snap = fetch_at(home.path(), SEVEN_RESET + 60).unwrap();
    let seven = snap.window(WindowKind::SevenDay).unwrap();
    assert_eq!(seven.used_pct, 0.0);
    assert_eq!(seven.resets_at, Some(at(SEVEN_RESET)));
    assert!(snap.stale);
    assert!(snap.stale_reason.unwrap().contains("6 hours"));
}

#[test]
fn recent_event_past_its_reset_is_fresh_and_ready() {
    let home = tempfile::tempdir().unwrap();
    let line = r#"{"timestamp":"2027-01-15T07:43:20Z","type":"event_msg","payload":{"type":"token_count","rate_limits":{"limit_id":"codex","primary":{"used_percent":80.0,"window_minutes":300,"resets_at":1799999500},"secondary":null}}}"#;
    put(home.path(), "2027/01/15/rollout-a.jsonl", line, EVENT_AT);
    let snap = fetch_at(home.path(), EVENT_AT + 600).unwrap();
    assert!(!snap.stale);
    assert_eq!(snap.window(WindowKind::FiveHour).unwrap().used_pct, 0.0);
}

#[test]
fn missing_sessions_folder_gives_a_stale_snapshot() {
    let home = tempfile::tempdir().unwrap();
    let snap = fetch_at(home.path(), EVENT_AT).unwrap();
    assert!(snap.stale);
    assert!(snap.windows.is_empty());
    assert!(snap.stale_reason.unwrap().contains("no session logs"));
}

#[test]
fn logs_without_an_event_give_a_stale_snapshot() {
    let home = tempfile::tempdir().unwrap();
    put(
        home.path(),
        "2027/01/15/rollout-a.jsonl",
        "{\"type\":\"session_meta\"}\n",
        EVENT_AT,
    );
    let snap = fetch_at(home.path(), EVENT_AT).unwrap();
    assert!(snap.stale);
    assert!(snap.stale_reason.unwrap().contains("no rate limit event"));
}

#[test]
fn wrong_provider_is_an_error() {
    let acct = Account::new("acc1", ProviderKind::ClaudeCache, None);
    let err = CodexLogsProvider::new(FixedClock(at(EVENT_AT)))
        .fetch(&acct)
        .unwrap_err();
    assert!(matches!(err, Error::WrongProvider { .. }));
}

fn event_line(ts: &str, pct: f64) -> String {
    format!(
        r#"{{"timestamp":"{ts}","type":"event_msg","payload":{{"type":"token_count","rate_limits":{{"limit_id":"codex","primary":{{"used_percent":{pct},"window_minutes":10080,"resets_at":1800300000}},"secondary":null}}}}}}"#
    )
}

fn newest_pct(home: &Path, cache: &ScanCache) -> f64 {
    let latest = newest_event(&sessions_dir(home), cache).unwrap().unwrap();
    latest.event.windows[0].used_pct
}

#[test]
fn newest_event_follows_the_modified_time_not_the_folder() {
    // A resumed session appends to the file in its old day folder.
    let home = tempfile::tempdir().unwrap();
    let old_day = event_line("2027-01-15T07:43:20Z", 61.0);
    let new_day = event_line("2027-01-15T07:00:00Z", 12.0);
    put(
        home.path(),
        "2027/01/10/rollout-a.jsonl",
        &old_day,
        EVENT_AT,
    );
    put(
        home.path(),
        "2027/01/15/rollout-b.jsonl",
        &new_day,
        EVENT_AT - 2_600,
    );
    assert_eq!(newest_pct(home.path(), &ScanCache::default()), 61.0);
}

#[test]
fn newest_event_skips_compressed_logs() {
    let home = tempfile::tempdir().unwrap();
    put(
        home.path(),
        "2027/01/15/rollout-a.jsonl",
        &event_line("2027-01-15T07:00:00Z", 12.0),
        EVENT_AT - 2_600,
    );
    put(
        home.path(),
        "2027/01/15/rollout-b.jsonl.zst",
        &event_line("2027-01-15T07:43:20Z", 99.0),
        EVENT_AT,
    );
    assert_eq!(newest_pct(home.path(), &ScanCache::default()), 12.0);
}

#[test]
fn newest_event_uses_the_file_time_without_a_timestamp() {
    let home = tempfile::tempdir().unwrap();
    let line = r#"{"type":"event_msg","payload":{"type":"token_count","rate_limits":{"primary":{"used_percent":5,"window_minutes":10080,"resets_at":1800300000}}}}"#;
    put(home.path(), "2027/01/15/rollout-a.jsonl", line, EVENT_AT);
    let latest = newest_event(&sessions_dir(home.path()), &ScanCache::default())
        .unwrap()
        .unwrap();
    assert_eq!(latest.at, at(EVENT_AT));
    assert_eq!(latest.event.at, None);
}

#[test]
fn cache_keeps_a_file_until_it_changes() {
    let home = tempfile::tempdir().unwrap();
    let path = put(
        home.path(),
        "2027/01/15/rollout-a.jsonl",
        &event_line("2027-01-15T07:43:20Z", 10.0),
        EVENT_AT,
    );
    let cache = ScanCache::default();
    assert_eq!(newest_pct(home.path(), &cache), 10.0);
    // The same size and time: the cache answers.
    let same = event_line("2027-01-15T07:43:20Z", 20.0);
    put(home.path(), "2027/01/15/rollout-a.jsonl", &same, EVENT_AT);
    assert_eq!(newest_pct(home.path(), &cache), 10.0);
    // A new time: the scan reads the file again.
    let body = std::fs::read_to_string(&path).unwrap();
    put(
        home.path(),
        "2027/01/15/rollout-a.jsonl",
        &body,
        EVENT_AT + 1,
    );
    assert_eq!(newest_pct(home.path(), &cache), 20.0);
}

#[test]
fn last_event_reads_across_block_edges_and_skips_long_lines() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rollout-a.jsonl");
    let filler = format!(
        "{{\"type\":\"response_item\",\"payload\":{{\"text\":\"{}\"}}}}\n",
        "x".repeat(40_000)
    );
    let long = format!("{{\"token_count\":\"{}\"}}\n", "y".repeat(1_500_000));
    let body = format!(
        "{}\n{}{}{}",
        event_line("2027-01-15T07:43:20Z", 44.0),
        filler.repeat(3),
        long,
        filler
    );
    std::fs::write(&path, body).unwrap();
    let event = last_event(&path).unwrap().unwrap();
    assert_eq!(event.windows[0].used_pct, 44.0);
}

#[test]
fn parse_line_reads_only_token_count_events() {
    let good = event_line("2027-01-15T07:43:20Z", 9.0);
    assert!(parse_line(good.as_bytes()).is_some());
    let other_type = good.replace("event_msg", "response_item");
    assert!(parse_line(other_type.as_bytes()).is_none());
    let no_windows = r#"{"type":"event_msg","payload":{"type":"token_count","rate_limits":{"limit_id":"codex","primary":null,"secondary":null}}}"#;
    assert!(parse_line(no_windows.as_bytes()).is_none());
    assert!(parse_line(b"not json token_count").is_none());
}

#[test]
fn codex_home_follows_the_environment_rule() {
    assert_eq!(
        codex_home_from(Some("/x/codex"), Some("/home/u")).unwrap(),
        PathBuf::from("/x/codex")
    );
    assert_eq!(
        codex_home_from(Some(""), Some("/home/u")).unwrap(),
        PathBuf::from("/home/u/.codex")
    );
    assert!(matches!(
        codex_home_from(None, None),
        Err(Error::NoCodexHome)
    ));
}
