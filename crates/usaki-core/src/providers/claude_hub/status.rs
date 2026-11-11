//! Parses the status JSON of a pool. The shape is in `docs/design/claude-hub.md`.

use serde_json::Value;

/// The state of one seat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatState {
    Open,
    Warm,
    FiveHourFull,
    WeekFull,
    Unknown,
}

impl SeatState {
    /// Maps the wire string. Any other value, or none, is `Unknown`.
    pub fn parse(value: Option<&str>) -> Self {
        match value {
            Some("open") => Self::Open,
            Some("warm") => Self::Warm,
            Some("five_hour_full") => Self::FiveHourFull,
            Some("week_full") => Self::WeekFull,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Warm => "warm",
            Self::FiveHourFull => "five_hour_full",
            Self::WeekFull => "week_full",
            Self::Unknown => "unknown",
        }
    }

    /// A seat that can take work: `open` or `warm`.
    pub fn is_available(self) -> bool {
        matches!(self, Self::Open | Self::Warm)
    }
}

/// One usage window of a seat. The percent and the reset (epoch seconds) can be missing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowStatus {
    pub used_pct: Option<f64>,
    pub reset_epoch: Option<i64>,
}

impl WindowStatus {
    /// Returns the window as it stands at `now`. A window whose reset passed counts as 0%.
    pub fn as_of(&self, now: i64) -> Self {
        match (self.used_pct, self.reset_epoch) {
            (Some(_), Some(reset)) if reset <= now => Self {
                used_pct: Some(0.0),
                ..*self
            },
            _ => *self,
        }
    }
}

/// One seat of a pool.
#[derive(Debug, Clone, PartialEq)]
pub struct SeatStatus {
    pub seat: String,
    pub priority: Option<i64>,
    pub state: SeatState,
    pub unavailable: bool,
    pub week: WindowStatus,
    pub five_hour: WindowStatus,
}

/// The status of one pool at `updated_epoch`.
#[derive(Debug, Clone, PartialEq)]
pub struct PoolStatus {
    pub updated_epoch: i64,
    pub fetched_at: Option<i64>,
    pub next_seat: Option<String>,
    pub extra_usage_open: Vec<String>,
    pub spend_guard_blocked: Vec<String>,
    pub seats: Vec<SeatStatus>,
}

/// Parses a status body. The error text never repeats the body.
pub fn parse_status(text: &str) -> Result<PoolStatus, String> {
    let root: Value = serde_json::from_str(text)
        .map_err(|e| format!("The pool status is not valid JSON. {e}"))?;
    let updated_epoch = epoch(root.get("updated_epoch"))
        .ok_or("The pool status has no valid updated_epoch field.")?;
    let seats = root
        .get("seats")
        .and_then(Value::as_array)
        .ok_or("The pool status has no seats array.")?
        .iter()
        .filter_map(|v| parse_seat(v, updated_epoch))
        .collect();
    Ok(PoolStatus {
        updated_epoch,
        fetched_at: epoch(root.get("fetched_at")),
        next_seat: root
            .get("next_seat")
            .and_then(Value::as_str)
            .map(str::to_owned),
        extra_usage_open: names(root.get("extra_usage_open")),
        spend_guard_blocked: names(root.get("spend_guard_blocked")),
        seats,
    })
}

fn parse_seat(v: &Value, updated: i64) -> Option<SeatStatus> {
    let seat = v.get("seat")?.as_str()?.to_owned();
    Some(SeatStatus {
        seat,
        priority: v.get("priority").and_then(Value::as_i64),
        state: SeatState::parse(v.get("state").and_then(Value::as_str)),
        unavailable: v
            .get("unavailable")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        week: window(v, "week", updated),
        five_hour: window(v, "five_hour", updated),
    })
}

/// Reads the window with the key prefix `prefix`: `<prefix>_used`, `_reset_epoch`, `_hours_left`.
fn window(v: &Value, prefix: &str, updated: i64) -> WindowStatus {
    let get = |suffix: &str| v.get(format!("{prefix}_{suffix}"));
    let used_pct = get("used")
        .and_then(Value::as_f64)
        .map(|f| (f * 100.0 * 1e6).round() / 1e6);
    let reset_epoch = epoch(get("reset_epoch")).or_else(|| {
        get("hours_left")
            .and_then(Value::as_f64)
            .map(|h| updated + (h * 3600.0).round() as i64)
    });
    WindowStatus {
        used_pct,
        reset_epoch,
    }
}

fn epoch(v: Option<&Value>) -> Option<i64> {
    let v = v?;
    v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))
}

fn names(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}
