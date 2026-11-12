//! Tests for the Claude hub provider. All names are synthetic.

use std::cell::{Cell, RefCell};
use std::path::Path;

use tempfile::TempDir;
use usaki_core::providers::claude_hub::forecast::{
    forecast, pool_total, recent_rate, seat_score, week_rate,
};
use usaki_core::providers::claude_hub::live::read_live;
use usaki_core::providers::claude_hub::status::{SeatState, SeatStatus, WindowStatus};
use usaki_core::providers::claude_hub::{
    Basis, DataSource, HubDirs, HubReader, PoolStatus, RateHistory, Sample, StatusClient,
    parse_status,
};
use usaki_core::providers::claude_watch::HttpReply;

const NOW: i64 = 1_800_000_060;
const KEY: &str = "sk-synthetic-key-123";
const FIX: &str = "tests/fixtures/hub";

fn fixture(name: &str) -> String {
    std::fs::read_to_string(Path::new(FIX).join(name)).unwrap()
}

// ---- parse ----

#[test]
fn parse_converts_fractions_and_reads_epochs() {
    let s = parse_status(&fixture("pool-a.status.json")).unwrap();
    assert_eq!(s.updated_epoch, 1_800_000_000);
    assert_eq!(s.fetched_at, Some(1_800_000_005));
    assert_eq!(s.next_seat.as_deref(), Some("acc1"));
    assert_eq!(s.extra_usage_open, ["acc3"]);
    assert!(s.spend_guard_blocked.is_empty());
    let a = &s.seats[0];
    assert_eq!(a.state, SeatState::Open);
    assert_eq!(a.priority, Some(100));
    assert_eq!(a.week.used_pct, Some(51.0));
    assert_eq!(a.week.reset_epoch, Some(1_800_418_680));
    assert_eq!(a.five_hour.used_pct, Some(6.0));
    assert_eq!(s.seats[1].state, SeatState::FiveHourFull);
    assert_eq!(s.seats[1].five_hour.used_pct, Some(100.0));
}

#[test]
fn parse_uses_hours_left_when_the_epoch_is_missing() {
    let s = parse_status(&fixture("hours-left-only.status.json")).unwrap();
    assert_eq!(s.seats[0].week.reset_epoch, Some(1_800_000_000 + 36_000));
    assert_eq!(s.seats[0].state, SeatState::Warm);
    assert_eq!(s.seats[0].week.used_pct, Some(25.0));
}

#[test]
fn parse_maps_unknown_and_missing_fields() {
    let s = parse_status(&fixture("hours-left-only.status.json")).unwrap();
    assert_eq!(s.seats[1].state, SeatState::Unknown);
    assert_eq!(s.seats[1].week.used_pct, None);
    assert_eq!(s.seats[2].state, SeatState::Unknown);
    assert_eq!(s.seats[2].week.reset_epoch, None);
    assert!(!s.seats[2].unavailable);
}

#[test]
fn parse_refuses_a_malformed_file() {
    let err = parse_status(&fixture("malformed.status.json")).unwrap_err();
    assert!(err.contains("not valid JSON"));
    assert!(parse_status(&fixture("no-seats-array.status.json")).is_err());
    assert!(parse_status(r#"{"seats":[]}"#).is_err());
}

#[test]
fn window_past_reset_counts_as_zero() {
    let w = WindowStatus {
        used_pct: Some(80.0),
        reset_epoch: Some(NOW - 1),
    };
    assert_eq!(w.as_of(NOW).used_pct, Some(0.0));
}

// ---- source rule ----

struct FakeClient {
    reply: RefCell<Result<HttpReply, String>>,
    calls: Cell<u32>,
    seen: RefCell<Vec<(String, String)>>,
}

impl FakeClient {
    fn ok(body: String) -> Self {
        Self::with(Ok(HttpReply { status: 200, body }))
    }
    fn with(reply: Result<HttpReply, String>) -> Self {
        Self {
            reply: RefCell::new(reply),
            calls: Cell::new(0),
            seen: RefCell::new(Vec::new()),
        }
    }
}

impl StatusClient for &FakeClient {
    fn get_status(&self, url: &str, key: &str) -> Result<HttpReply, String> {
        self.calls.set(self.calls.get() + 1);
        self.seen
            .borrow_mut()
            .push((url.to_owned(), key.to_owned()));
        self.reply.borrow().clone()
    }
}

struct Env {
    _conf: TempDir,
    _cache: TempDir,
    dirs: HubDirs,
}

fn env() -> Env {
    let conf = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let dirs = HubDirs::new(conf.path(), cache.path());
    Env {
        _conf: conf,
        _cache: cache,
        dirs,
    }
}

fn put_cache(env: &Env, pool: &str, body: &str) {
    std::fs::write(env.dirs.cache.join(format!("{pool}.status.json")), body).unwrap();
}

fn put_endpoint(env: &Env, pool: &str, status_url: &str, pool_url: &str) {
    let c = &env.dirs.conf;
    std::fs::write(
        c.join(format!("{pool}.status-url")),
        format!("{status_url}\n"),
    )
    .unwrap();
    std::fs::write(c.join(format!("{pool}.url")), format!("{pool_url}\n")).unwrap();
    std::fs::write(c.join(format!("{pool}.key")), format!("  {KEY}  \n")).unwrap();
}

/// A body of one seat, updated at `updated`.
fn body(updated: i64, week_used: f64) -> String {
    format!(
        r#"{{"updated_epoch":{updated},"seats":[{{"seat":"acc1","state":"open","week_used":{week_used},"week_reset_epoch":{r}}}]}}"#,
        r = updated + 100_000
    )
}

#[test]
fn discovery_finds_pools_by_cache_file_or_endpoint_config() {
    let e = env();
    put_cache(&e, "pool-a", &body(NOW, 0.1));
    put_endpoint(&e, "pool-b", "https://hub.example/s", "https://hub.example");
    // A status URL without a key is no pool. A bad name is skipped.
    std::fs::write(
        e.dirs.conf.join("pool-c.status-url"),
        "https://hub.example/s",
    )
    .unwrap();
    put_cache(&e, "Bad_Name", &body(NOW, 0.1));
    assert_eq!(e.dirs.discover_pools(), ["pool-a", "pool-b"]);
}

#[test]
fn fresh_cache_is_used_without_a_call() {
    let e = env();
    put_cache(&e, "pool-a", &body(NOW - 120, 0.4));
    put_endpoint(&e, "pool-a", "https://hub.example/s", "https://hub.example");
    let client = FakeClient::ok(body(NOW, 0.9));
    let mut reader = HubReader::new(e.dirs.clone(), &client);
    let r = reader.read_pool("pool-a", NOW);
    assert_eq!(client.calls.get(), 0);
    assert_eq!(r.source, DataSource::Cache);
    assert!(!r.stale);
    assert_eq!(r.error, None);
    assert_eq!(r.total.unwrap().seven_day_pct, Some(40.0));
}

#[test]
fn stale_cache_falls_back_to_the_endpoint() {
    let e = env();
    put_cache(&e, "pool-a", &body(NOW - 3000, 0.4));
    put_endpoint(
        &e,
        "pool-a",
        "https://hub.example/s",
        "https://hub.example/pool",
    );
    let client = FakeClient::ok(body(NOW - 5, 0.5));
    let mut reader = HubReader::new(e.dirs.clone(), &client);
    let r = reader.read_pool("pool-a", NOW);
    assert_eq!(client.calls.get(), 1);
    assert_eq!(r.source, DataSource::Endpoint);
    assert!(!r.stale);
    assert_eq!(r.fetched_at, NOW);
    let seen = client.seen.borrow();
    assert_eq!(
        seen[0],
        ("https://hub.example/s".to_owned(), KEY.to_owned())
    );
}

#[test]
fn throttle_allows_one_call_per_minute_and_keeps_the_reply() {
    let e = env();
    put_endpoint(&e, "pool-a", "https://hub.example/s", "https://hub.example");
    let client = FakeClient::ok(body(NOW - 5, 0.5));
    let mut reader = HubReader::new(e.dirs.clone(), &client);
    reader.read_pool("pool-a", NOW);
    let r = reader.read_pool("pool-a", NOW + 30);
    assert_eq!(client.calls.get(), 1);
    assert_eq!(
        r.source,
        DataSource::Endpoint,
        "the last reply stays in memory"
    );
    reader.read_pool("pool-a", NOW + 61);
    assert_eq!(client.calls.get(), 2);
}

#[test]
fn newer_cache_beats_an_older_endpoint_reply() {
    let e = env();
    put_endpoint(&e, "pool-a", "https://hub.example/s", "https://hub.example");
    let client = FakeClient::ok(body(NOW - 200, 0.5));
    let mut reader = HubReader::new(e.dirs.clone(), &client);
    reader.read_pool("pool-a", NOW);
    put_cache(&e, "pool-a", &body(NOW - 10, 0.6));
    let r = reader.read_pool("pool-a", NOW + 1);
    assert_eq!(r.source, DataSource::Cache);
}

#[test]
fn host_mismatch_refuses_the_call() {
    let e = env();
    put_endpoint(
        &e,
        "pool-a",
        "https://other.example/s",
        "https://hub.example",
    );
    let client = FakeClient::ok(body(NOW, 0.5));
    let mut reader = HubReader::new(e.dirs.clone(), &client);
    let r = reader.read_pool("pool-a", NOW);
    assert_eq!(client.calls.get(), 0);
    assert!(r.stale && r.status.is_none());
    assert!(r.error.unwrap().contains("host differs"));
}

#[test]
fn plain_http_status_url_is_refused() {
    let e = env();
    put_endpoint(&e, "pool-a", "http://hub.example/s", "http://hub.example");
    let client = FakeClient::ok(body(NOW, 0.5));
    let mut reader = HubReader::new(e.dirs.clone(), &client);
    reader.read_pool("pool-a", NOW);
    assert_eq!(client.calls.get(), 0);
}

#[test]
fn the_key_never_appears_in_an_error() {
    let e = env();
    put_endpoint(&e, "pool-a", "https://hub.example/s", "https://hub.example");
    let client = FakeClient::with(Err(format!("The status call failed. header {KEY}")));
    let mut reader = HubReader::new(e.dirs.clone(), &client);
    let r = reader.read_pool("pool-a", NOW);
    let error = r.error.unwrap();
    assert!(!error.contains(KEY), "{error}");
    assert!(error.contains("[key]"));
    // A bad HTTP status and a bad body give an error without the key too.
    let bad = FakeClient::with(Ok(HttpReply {
        status: 500,
        body: KEY.to_owned(),
    }));
    let mut reader = HubReader::new(e.dirs.clone(), &bad);
    let error = reader.read_pool("pool-a", NOW).error.unwrap();
    assert!(error.contains("HTTP 500") && !error.contains(KEY));
    let junk = FakeClient::ok(KEY.to_owned());
    let mut reader = HubReader::new(e.dirs.clone(), &junk);
    let error = reader.read_pool("pool-a", NOW).error.unwrap();
    assert!(!error.contains(KEY));
}

#[test]
fn stale_data_stays_with_the_error_when_the_call_fails() {
    let e = env();
    put_cache(&e, "pool-a", &body(NOW - 3000, 0.4));
    put_endpoint(&e, "pool-a", "https://hub.example/s", "https://hub.example");
    let client = FakeClient::with(Err("The status call failed. timeout".into()));
    let mut reader = HubReader::new(e.dirs.clone(), &client);
    let r = reader.read_pool("pool-a", NOW);
    assert!(r.stale);
    assert!(r.status.is_some());
    assert!(r.error.unwrap().contains("timeout"));
    assert_eq!(r.updated_epoch, NOW - 3000);
}

#[test]
fn old_cache_without_endpoint_is_stale() {
    let e = env();
    put_cache(&e, "pool-a", &body(NOW - 601, 0.4));
    let client = FakeClient::ok(String::new());
    let mut reader = HubReader::new(e.dirs.clone(), &client);
    let r = reader.read_pool("pool-a", NOW);
    assert!(r.stale);
    assert!(r.error.unwrap().contains("older than 10 minutes"));
    put_cache(&e, "pool-a", &body(NOW - 600, 0.4));
    assert!(!reader.read_pool("pool-a", NOW).stale);
}

#[test]
fn malformed_cache_gives_a_pool_with_an_error() {
    let e = env();
    put_cache(&e, "pool-a", &fixture("malformed.status.json"));
    let client = FakeClient::ok(String::new());
    let mut reader = HubReader::new(e.dirs.clone(), &client);
    let r = reader.read_pool("pool-a", NOW);
    assert!(r.stale && r.status.is_none() && r.total.is_none());
    assert_eq!(r.updated_epoch, NOW);
    assert!(r.error.unwrap().contains("not valid JSON"));
}

// ---- live sessions ----

#[test]
fn live_sessions_count_recent_files_for_each_seat() {
    let e = env();
    assert_eq!(read_live(&e.dirs, "pool-a", NOW), None);
    let live = e.dirs.cache.join("pool-a.live");
    std::fs::create_dir(&live).unwrap();
    let w = |name: &str, text: String| std::fs::write(live.join(name), text).unwrap();
    w(
        "s1.json",
        format!(r#"{{"seat":"acc1","written_at":{}}}"#, NOW - 600),
    );
    w(
        "s2.json",
        format!(r#"{{"seat":"acc1","written_at":{}}}"#, NOW - 10),
    );
    w(
        "s3.json",
        format!(r#"{{"seat":"acc2","written_at":{}}}"#, NOW - 601),
    );
    w("s4.json", "{ bad".to_owned());
    w("notes.txt", "x".to_owned());
    let counts = read_live(&e.dirs, "pool-a", NOW).unwrap();
    assert_eq!(counts.get("acc1"), Some(&2));
    assert_eq!(counts.get("acc2"), None);
}

// ---- forecast ----

const HOUR: i64 = 3600;

fn seat(name: &str, pct: f64, reset: Option<i64>) -> SeatStatus {
    SeatStatus {
        seat: name.to_owned(),
        priority: None,
        state: SeatState::Open,
        unavailable: false,
        week: WindowStatus {
            used_pct: Some(pct),
            reset_epoch: reset,
        },
        five_hour: WindowStatus {
            used_pct: None,
            reset_epoch: None,
        },
    }
}

fn status(updated: i64, seats: Vec<SeatStatus>) -> PoolStatus {
    PoolStatus {
        updated_epoch: updated,
        fetched_at: None,
        next_seat: None,
        extra_usage_open: Vec::new(),
        spend_guard_blocked: Vec::new(),
        seats,
    }
}

#[test]
fn totals_average_the_seats_and_zero_a_passed_reset() {
    let mut s = status(
        NOW,
        vec![
            seat("acc1", 50.0, Some(NOW + 10 * HOUR)),
            seat("acc2", 90.0, Some(NOW - 1)),
            seat("acc3", 10.0, Some(NOW + 5 * HOUR)),
        ],
    );
    s.seats[2].state = SeatState::WeekFull;
    let t = pool_total(&s, NOW);
    assert_eq!(t.seven_day_pct, Some(20.0));
    assert_eq!(t.five_hour_pct, None);
    assert_eq!((t.open_seats, t.seat_count), (2, 3));
    assert_eq!(t.next_refill_at, Some(NOW + 5 * HOUR));
}

#[test]
fn seat_score_is_percent_left_per_hour() {
    assert_eq!(
        seat_score(Some(60.0), Some(NOW + 10 * HOUR), NOW),
        Some(4.0)
    );
    assert_eq!(seat_score(Some(60.0), Some(NOW - 1), NOW), None);
    assert_eq!(seat_score(None, Some(NOW + HOUR), NOW), None);
}

#[test]
fn rate_zero_never_runs_out() {
    let s = status(NOW, vec![seat("acc1", 0.0, Some(NOW + 100 * HOUR))]);
    let f = forecast(&s, &[], NOW).unwrap();
    assert_eq!(f.rate_pct_per_hour, 0.0);
    assert_eq!(f.basis, Basis::Week);
    assert_eq!(f.runs_out_at, None);
}

#[test]
fn week_rate_is_percent_per_elapsed_hour() {
    // 84 h left, so 84 h elapsed. 42% over 84 h is 0.5 per hour.
    let s = status(NOW, vec![seat("acc1", 42.0, Some(NOW + 84 * HOUR))]);
    assert!((week_rate(&s, NOW) - 0.5).abs() < 1e-9);
    // A seat with under 1 h elapsed is skipped.
    let fresh = status(NOW, vec![seat("acc1", 5.0, Some(NOW + 167 * HOUR + 1800))]);
    assert_eq!(week_rate(&fresh, NOW), 0.0);
}

#[test]
fn pool_runs_out_before_the_reset() {
    // 20% left at 1.0 per hour is 20 h, before the reset at 50 h.
    let samples = vec![
        sample(NOW - 1800, &[("acc1", 79.5, NOW + 50 * HOUR)]),
        sample(NOW, &[("acc1", 80.0, NOW + 50 * HOUR)]),
    ];
    let s = status(NOW, vec![seat("acc1", 80.0, Some(NOW + 50 * HOUR))]);
    let f = forecast(&s, &samples, NOW).unwrap();
    assert_eq!(f.basis, Basis::Recent);
    assert!((f.rate_pct_per_hour - 1.0).abs() < 1e-9);
    assert_eq!(f.runs_out_at, Some(NOW + 20 * HOUR));
}

#[test]
fn a_reset_that_refills_pushes_the_run_out_later() {
    // Two seats at 50% each, rate 10 per hour, demand 20 seat percent per hour.
    // Without a reset the pool is out in 100 / 20 = 5 h.
    let samples = vec![
        sample(
            NOW - 1800,
            &[
                ("acc1", 45.0, NOW + 3 * HOUR),
                ("acc2", 45.0, NOW + 90 * HOUR),
            ],
        ),
        sample(
            NOW,
            &[
                ("acc1", 50.0, NOW + 3 * HOUR),
                ("acc2", 50.0, NOW + 90 * HOUR),
            ],
        ),
    ];
    let no_reset = status(
        NOW,
        vec![
            seat("acc1", 50.0, Some(NOW + 90 * HOUR)),
            seat("acc2", 50.0, Some(NOW + 90 * HOUR)),
        ],
    );
    let samples_no_reset = vec![
        sample(
            NOW - 1800,
            &[
                ("acc1", 45.0, NOW + 90 * HOUR),
                ("acc2", 45.0, NOW + 90 * HOUR),
            ],
        ),
        sample(
            NOW,
            &[
                ("acc1", 50.0, NOW + 90 * HOUR),
                ("acc2", 50.0, NOW + 90 * HOUR),
            ],
        ),
    ];
    let early = forecast(&no_reset, &samples_no_reset, NOW).unwrap();
    assert_eq!(early.runs_out_at, Some(NOW + 5 * HOUR));
    let with_reset = status(
        NOW,
        vec![
            seat("acc1", 50.0, Some(NOW + 3 * HOUR)),
            seat("acc2", 50.0, Some(NOW + 90 * HOUR)),
        ],
    );
    let late = forecast(&with_reset, &samples, NOW).unwrap();
    // At 3 h the pool drained 60 of 100. Seat acc1 refills, so 40 + 100 = 140 left. 140 / 20 = 7 h.
    assert_eq!(late.runs_out_at, Some(NOW + 10 * HOUR));
}

#[test]
fn run_out_beyond_the_horizon_is_none() {
    let samples = vec![
        sample(NOW - 1800, &[("acc1", 0.0, NOW + 160 * HOUR)]),
        sample(NOW, &[("acc1", 0.05, NOW + 160 * HOUR)]),
    ];
    // Rate 0.1 per hour (0.05 in 30 min). The seat resets at 160 h and refills, but the pool still lasts past 168 h.
    let s = status(NOW, vec![seat("acc1", 0.2, Some(NOW + 160 * HOUR))]);
    let f = forecast(&s, &samples, NOW).unwrap();
    assert_eq!(f.runs_out_at, None);
}

fn sample(updated: i64, seats: &[(&str, f64, i64)]) -> Sample {
    sample_of(&status(
        updated,
        seats
            .iter()
            .map(|(n, p, r)| seat(n, *p, Some(*r)))
            .collect(),
    ))
}

fn sample_of(s: &PoolStatus) -> Sample {
    Sample::from_status(s)
}

#[test]
fn recent_rate_needs_20_minutes_and_ignores_a_changed_reset() {
    let a = sample(NOW - 900, &[("acc1", 10.0, NOW + HOUR)]);
    let b = sample(NOW, &[("acc1", 14.0, NOW + HOUR)]);
    assert_eq!(recent_rate(&[a, b], NOW), None, "15 min span is too short");
    let a = sample(
        NOW - 1800,
        &[("acc1", 10.0, NOW + HOUR), ("acc2", 20.0, NOW + HOUR)],
    );
    let b = sample(
        NOW,
        &[("acc1", 14.0, NOW + HOUR), ("acc2", 3.0, NOW + 168 * HOUR)],
    );
    // acc1 gained 4 in 0.5 h. acc2 reset, so it is skipped. Mean over 2 seats: 8 / 2 = 4.
    assert!((recent_rate(&[a, b], NOW).unwrap() - 4.0).abs() < 1e-9);
}

#[test]
fn recent_rate_differs_from_the_week_rate() {
    let s = status(NOW, vec![seat("acc1", 42.0, Some(NOW + 84 * HOUR))]);
    let samples = vec![
        sample(NOW - 1800, &[("acc1", 40.0, NOW + 84 * HOUR)]),
        sample(NOW, &[("acc1", 42.0, NOW + 84 * HOUR)]),
    ];
    let f = forecast(&s, &samples, NOW).unwrap();
    assert_eq!(f.basis, Basis::Recent);
    assert!((f.rate_pct_per_hour - 4.0).abs() < 1e-9);
    assert!((week_rate(&s, NOW) - 0.5).abs() < 1e-9);
}

#[test]
fn history_records_new_epochs_only_and_drops_old_samples() {
    let mut h = RateHistory::default();
    let at = |t: i64| status(t, vec![seat("acc1", 1.0, Some(t + HOUR))]);
    h.record("pool-a", &at(NOW - 6000), NOW - 6000);
    h.record("pool-a", &at(NOW - 6000), NOW - 5990);
    assert_eq!(h.samples("pool-a").len(), 1);
    h.record("pool-a", &at(NOW), NOW);
    let epochs: Vec<i64> = h
        .samples("pool-a")
        .iter()
        .map(|s| s.updated_epoch)
        .collect();
    assert_eq!(epochs, [NOW], "the sample older than 90 min is gone");
}
