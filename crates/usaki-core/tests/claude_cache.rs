//! Tests for the Claude cache provider. Fixture data is synthetic.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, TimeZone, Utc};
use usaki_core::providers::claude_cache::{
    ClaudeCacheProvider, cache_dir_from, cache_dir_from_env, cache_path, parse_snapshot,
};
use usaki_core::{Account, Clock, Error, Provider, ProviderKind, Source, WindowKind};

const WRITTEN_AT: i64 = 1_800_000_000;
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
        .join("tests/fixtures")
        .join(name)
}

fn fetch_at(name: &str, now: DateTime<Utc>) -> usaki_core::Result<usaki_core::Snapshot> {
    let acct = Account::new("acc1", ProviderKind::ClaudeCache, Some(fixture(name)));
    ClaudeCacheProvider::new(FixedClock(now)).fetch(&acct)
}

fn fetch(name: &str) -> usaki_core::Result<usaki_core::Snapshot> {
    fetch_at(name, at(WRITTEN_AT + 60))
}

#[test]
fn flat_shape_parses() {
    let snap = fetch("flat.json").unwrap();
    assert_eq!(snap.account_id, "acc1");
    assert_eq!(snap.source, Source::ClaudeCache);
    assert!(!snap.stale);
    assert_eq!(snap.fetched_at, at(WRITTEN_AT));
    let five = snap.window(WindowKind::FiveHour).unwrap();
    assert_eq!(five.used_pct, 42.0);
    assert_eq!(five.resets_at, Some(at(FIVE_RESET)));
    let seven = snap.window(WindowKind::SevenDay).unwrap();
    assert_eq!(seven.used_pct, 17.0);
    assert_eq!(seven.resets_at, Some(at(SEVEN_RESET)));
}

#[test]
fn limits_shape_parses_and_maps_minutes() {
    let snap = fetch("limits.json").unwrap();
    assert_eq!(snap.windows.len(), 3);
    assert_eq!(snap.window(WindowKind::FiveHour).unwrap().used_pct, 61.0);
    assert_eq!(snap.window(WindowKind::SevenDay).unwrap().used_pct, 33.0);
    assert_eq!(snap.window(WindowKind::Other(60)).unwrap().used_pct, 5.0);
}

#[test]
fn missing_window_is_absent() {
    let snap = fetch("missing_window.json").unwrap();
    assert_eq!(snap.windows.len(), 1);
    assert!(snap.window(WindowKind::SevenDay).is_none());
}

#[test]
fn float_percent_and_float_epoch_parse() {
    let snap = fetch("float_percent.json").unwrap();
    let five = snap.window(WindowKind::FiveHour).unwrap();
    assert_eq!(five.used_pct, 42.5);
    assert_eq!(five.resets_at, Some(at(FIVE_RESET)));
    assert_eq!(snap.window(WindowKind::SevenDay).unwrap().used_pct, 17.25);
}

#[test]
fn iso_resets_at_parses() {
    let snap = fetch("iso_resets.json").unwrap();
    assert_eq!(
        snap.window(WindowKind::FiveHour).unwrap().resets_at,
        Some(at(FIVE_RESET))
    );
    assert_eq!(
        snap.window(WindowKind::SevenDay).unwrap().resets_at,
        Some(at(SEVEN_RESET))
    );
}

#[test]
fn malformed_file_is_an_error_with_a_clear_message() {
    let err = fetch("malformed.json").unwrap_err();
    assert!(matches!(err, Error::Malformed { .. }));
    let msg = err.to_string();
    assert!(msg.contains("malformed.json"), "{msg}");
    assert!(msg.contains("not valid JSON"), "{msg}");
}

#[test]
fn bad_field_type_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("claude-acc1.json");
    std::fs::write(
        &path,
        r#"{"five_hour":{"used_percentage":"high","resets_at":1},"written_at":1800000000}"#,
    )
    .unwrap();
    let acct = Account::new("acc1", ProviderKind::ClaudeCache, Some(path));
    let err = ClaudeCacheProvider::new(FixedClock(at(WRITTEN_AT)))
        .fetch(&acct)
        .unwrap_err();
    assert!(matches!(err, Error::InvalidField { .. }), "{err}");
    assert!(err.to_string().contains("five_hour.used_percentage"));
}

#[test]
fn missing_file_is_a_stale_empty_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let acct = Account::new(
        "acc1",
        ProviderKind::ClaudeCache,
        Some(dir.path().join("claude-acc1.json")),
    );
    let now = at(WRITTEN_AT);
    let snap = ClaudeCacheProvider::new(FixedClock(now))
        .fetch(&acct)
        .unwrap();
    assert!(snap.stale);
    assert!(snap.windows.is_empty());
    assert_eq!(snap.account_id, "acc1");
    assert_eq!(snap.fetched_at, now);
}

#[test]
fn exact_threshold_is_not_stale() {
    let now = at(WRITTEN_AT) + Duration::minutes(15);
    assert!(!fetch_at("flat.json", now).unwrap().stale);
}

#[test]
fn older_than_default_threshold_is_stale() {
    let now = at(WRITTEN_AT) + Duration::minutes(15) + Duration::seconds(1);
    assert!(fetch_at("flat.json", now).unwrap().stale);
}

#[test]
fn threshold_is_configurable() {
    let acct = Account::new(
        "acc1",
        ProviderKind::ClaudeCache,
        Some(fixture("flat.json")),
    );
    let now = at(WRITTEN_AT) + Duration::minutes(5);
    let provider = ClaudeCacheProvider::new(FixedClock(now)).with_stale_after(Duration::minutes(1));
    assert!(provider.fetch(&acct).unwrap().stale);
}

#[test]
fn past_reset_reports_zero_and_keeps_snapshot() {
    let now = at(FIVE_RESET + 10);
    let snap = fetch_at("flat.json", now).unwrap();
    let five = snap.window(WindowKind::FiveHour).unwrap();
    assert_eq!(five.used_pct, 0.0);
    assert_eq!(five.resets_at, Some(at(FIVE_RESET)));
    assert_eq!(snap.window(WindowKind::SevenDay).unwrap().used_pct, 17.0);
}

#[test]
fn missing_written_at_is_stale() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("c.json");
    std::fs::write(
        &path,
        r#"{"five_hour":{"used_percentage":1,"resets_at":1900000000}}"#,
    )
    .unwrap();
    let acct = Account::new("acc1", ProviderKind::ClaudeCache, Some(path));
    let snap = ClaudeCacheProvider::new(FixedClock(at(WRITTEN_AT)))
        .fetch(&acct)
        .unwrap();
    assert!(snap.stale);
    assert_eq!(snap.windows.len(), 1);
}

#[test]
fn default_path_uses_the_account_id() {
    let dir = cache_dir_from(Some("/x/cache"), Some("/home/u")).unwrap();
    assert_eq!(
        cache_path(&dir, "acc2"),
        PathBuf::from("/x/cache/usaki/claude-acc2.json")
    );
}

#[test]
fn cache_dir_falls_back_to_home() {
    assert_eq!(
        cache_dir_from(None, Some("/home/u")).unwrap(),
        PathBuf::from("/home/u/.cache")
    );
    assert_eq!(
        cache_dir_from(Some(""), Some("/home/u")).unwrap(),
        PathBuf::from("/home/u/.cache")
    );
    assert!(matches!(cache_dir_from(None, None), Err(Error::NoCacheDir)));
}

#[test]
fn provider_with_cache_dir_reads_the_default_path() {
    let dir = tempfile::tempdir().unwrap();
    let usaki = dir.path().join("usaki");
    std::fs::create_dir_all(&usaki).unwrap();
    std::fs::copy(fixture("flat.json"), usaki.join("claude-acc1.json")).unwrap();
    let acct = Account::new("acc1", ProviderKind::ClaudeCache, None);
    let snap = ClaudeCacheProvider::new(FixedClock(at(WRITTEN_AT)))
        .with_cache_dir(dir.path().to_path_buf())
        .fetch(&acct)
        .unwrap();
    assert!(snap.window(WindowKind::FiveHour).is_some());
}

fn parse(text: &str, now_secs: i64) -> usaki_core::Result<usaki_core::Snapshot> {
    parse_snapshot(
        "acc1",
        text,
        Path::new("inline.json"),
        at(now_secs),
        Duration::minutes(15),
    )
}

fn field_of(err: Error) -> String {
    match err {
        Error::InvalidField { field, .. } => field,
        other => panic!("expected InvalidField, got {other}"),
    }
}

#[test]
fn future_written_at_within_skew_is_fresh_and_clamped() {
    let text = r#"{"five_hour":{"used_percentage":1},"written_at":1800000060}"#;
    let snap = parse(text, WRITTEN_AT).unwrap();
    assert!(!snap.stale);
    assert_eq!(snap.fetched_at, at(WRITTEN_AT));
}

#[test]
fn future_written_at_beyond_skew_is_stale() {
    let text = r#"{"five_hour":{"used_percentage":1},"written_at":1800000121}"#;
    let snap = parse(text, WRITTEN_AT).unwrap();
    assert!(snap.stale);
    assert_eq!(snap.fetched_at, at(WRITTEN_AT));
}

#[test]
fn written_at_in_milliseconds_is_stale() {
    let text = r#"{"five_hour":{"used_percentage":1},"written_at":1800000000000}"#;
    let snap = parse(text, WRITTEN_AT).unwrap();
    assert!(snap.stale);
    assert_eq!(snap.fetched_at, at(WRITTEN_AT));
}

#[test]
fn negative_used_percentage_is_an_error() {
    let text = r#"{"five_hour":{"used_percentage":-20},"written_at":1800000000}"#;
    assert_eq!(
        field_of(parse(text, WRITTEN_AT).unwrap_err()),
        "five_hour.used_percentage"
    );
}

#[test]
fn used_percentage_over_100_is_kept() {
    let text = r#"{"five_hour":{"used_percentage":250},"written_at":1800000000}"#;
    let snap = parse(text, WRITTEN_AT).unwrap();
    assert_eq!(snap.window(WindowKind::FiveHour).unwrap().used_pct, 250.0);
}

#[test]
fn duplicate_window_minutes_is_an_error() {
    let text = r#"{"limits":[
        {"window_minutes":300,"used_percentage":5},
        {"window_minutes":300,"used_percentage":90}],"written_at":1800000000}"#;
    assert_eq!(
        field_of(parse(text, WRITTEN_AT).unwrap_err()),
        "limits[1].window_minutes"
    );
}

#[test]
fn limits_wins_over_flat_fields() {
    let text = r#"{"five_hour":{"used_percentage":1},
        "limits":[{"window_minutes":10080,"used_percentage":9}],"written_at":1800000000}"#;
    let snap = parse(text, WRITTEN_AT).unwrap();
    assert_eq!(snap.windows.len(), 1);
    assert!(snap.window(WindowKind::FiveHour).is_none());
    assert_eq!(snap.window(WindowKind::SevenDay).unwrap().used_pct, 9.0);
}

#[test]
fn empty_limits_gives_no_windows() {
    let snap = parse(r#"{"limits":[],"written_at":1800000000}"#, WRITTEN_AT).unwrap();
    assert!(snap.windows.is_empty());
}

#[test]
fn limits_that_is_not_an_array_is_an_error() {
    assert_eq!(
        field_of(parse(r#"{"limits":{}}"#, WRITTEN_AT).unwrap_err()),
        "limits"
    );
}

#[test]
fn root_that_is_not_an_object_is_an_error() {
    assert_eq!(field_of(parse("[1]", WRITTEN_AT).unwrap_err()), "(root)");
}

#[test]
fn bad_time_strings_are_errors() {
    let reset = r#"{"five_hour":{"used_percentage":1,"resets_at":"soon"}}"#;
    assert_eq!(
        field_of(parse(reset, WRITTEN_AT).unwrap_err()),
        "five_hour.resets_at"
    );
    let written = r#"{"five_hour":{"used_percentage":1},"written_at":"yesterday"}"#;
    assert_eq!(
        field_of(parse(written, WRITTEN_AT).unwrap_err()),
        "written_at"
    );
}

#[test]
fn non_integer_window_minutes_is_an_error() {
    for v in ["300.5", "\"300\"", "-5"] {
        let text = format!(r#"{{"limits":[{{"window_minutes":{v},"used_percentage":1}}]}}"#);
        assert_eq!(
            field_of(parse(&text, WRITTEN_AT).unwrap_err()),
            "limits[0].window_minutes"
        );
    }
}

#[test]
fn unreadable_path_is_an_io_error() {
    // A directory at the cache file path cannot be read as text.
    let dir = tempfile::tempdir().unwrap();
    let acct = Account::new(
        "acc1",
        ProviderKind::ClaudeCache,
        Some(dir.path().to_path_buf()),
    );
    let err = ClaudeCacheProvider::new(FixedClock(at(WRITTEN_AT)))
        .fetch(&acct)
        .unwrap_err();
    assert!(matches!(err, Error::Io { .. }), "{err}");
}

#[test]
fn account_id_with_a_path_separator_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    for id in ["../x", "a/b", "a\\b", "", "a.b"] {
        let acct = Account::new(id, ProviderKind::ClaudeCache, None);
        let err = ClaudeCacheProvider::new(FixedClock(at(WRITTEN_AT)))
            .with_cache_dir(dir.path().to_path_buf())
            .fetch(&acct)
            .unwrap_err();
        assert!(matches!(err, Error::InvalidAccountId { .. }), "{id}: {err}");
    }
}

#[test]
fn cache_dir_from_env_matches_cache_dir_from() {
    let xdg = std::env::var("XDG_CACHE_HOME").ok();
    let home = std::env::var("HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("USERPROFILE").ok());
    let expected = cache_dir_from(xdg.as_deref(), home.as_deref()).ok();
    assert_eq!(cache_dir_from_env().ok(), expected);
}
