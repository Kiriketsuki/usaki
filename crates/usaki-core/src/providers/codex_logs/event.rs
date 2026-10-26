//! Parses one line of a Codex session log into a rate limit event.
//!
//! The session logs hold whole conversations. The parser reads only the `rate_limits`
//! object of a `token_count` event. The typed structs below name no other field, so serde
//! keeps no conversation text. A line that does not parse gives `None`, and no error text
//! leaves this module, because a serde error can quote the line.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::model::{Credits, Window, kind_from_minutes};

/// The `limit_id` of the Codex plan limit. Other ids are other limits, such as a model limit.
pub const CODEX_LIMIT_ID: &str = "codex";

/// The bytes that every `token_count` line holds. The parser skips any other line unread.
const TOKEN_COUNT: &[u8] = b"token_count";

/// One rate limit event from the logs.
#[derive(Debug, Clone, PartialEq)]
pub struct CodexEvent {
    /// The `timestamp` of the line, when it has one.
    pub at: Option<DateTime<Utc>>,
    pub windows: Vec<Window>,
    pub credits: Option<Credits>,
    pub plan_type: Option<String>,
}

#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type")]
    kind: String,
    timestamp: Option<String>,
    payload: Option<Payload>,
}

#[derive(Deserialize)]
struct Payload {
    #[serde(rename = "type")]
    kind: Option<String>,
    rate_limits: Option<RateLimits>,
}

#[derive(Deserialize)]
struct RateLimits {
    limit_id: Option<String>,
    primary: Option<RawWindow>,
    secondary: Option<RawWindow>,
    credits: Option<RawCredits>,
    plan_type: Option<String>,
}

#[derive(Deserialize)]
struct RawWindow {
    used_percent: f64,
    window_minutes: u64,
    resets_at: Option<f64>,
}

#[derive(Deserialize)]
struct RawCredits {
    #[serde(default)]
    has_credits: bool,
    #[serde(default)]
    unlimited: bool,
    balance: Option<String>,
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Returns the rate limit event of one log line, or `None` for any other line.
///
/// The line must be an `event_msg` with a `token_count` payload and a non-null
/// `rate_limits`. The `limit_id` must be `codex`. A missing `limit_id` also counts as
/// `codex`, because a log without the field knows only the one plan limit. The windows come
/// from `primary` and `secondary` by `window_minutes`, never by slot. The event must hold at
/// least one window.
pub fn parse_line(line: &[u8]) -> Option<CodexEvent> {
    if !contains(line, TOKEN_COUNT) {
        return None;
    }
    let line: Line = serde_json::from_slice(line).ok()?;
    let payload = line.payload?;
    if line.kind != "event_msg" || payload.kind.as_deref() != Some("token_count") {
        return None;
    }
    let limits = payload.rate_limits?;
    if limits
        .limit_id
        .as_deref()
        .is_some_and(|id| id != CODEX_LIMIT_ID)
    {
        return None;
    }
    let windows = windows(&limits)?;
    if windows.is_empty() {
        return None;
    }
    let at = match line.timestamp {
        Some(t) => Some(DateTime::parse_from_rfc3339(&t).ok()?.with_timezone(&Utc)),
        None => None,
    };
    Some(CodexEvent {
        at,
        windows,
        credits: limits.credits.map(|c| Credits {
            has_credits: c.has_credits,
            unlimited: c.unlimited,
            balance: c.balance,
        }),
        plan_type: limits.plan_type,
    })
}

// Returns `None` when a window is bad or when two windows have the same length.
fn windows(limits: &RateLimits) -> Option<Vec<Window>> {
    let mut out: Vec<Window> = Vec::new();
    for raw in [&limits.primary, &limits.secondary].into_iter().flatten() {
        let window = window(raw)?;
        if out.iter().any(|w| w.kind == window.kind) {
            return None;
        }
        out.push(window);
    }
    Some(out)
}

fn window(raw: &RawWindow) -> Option<Window> {
    // A value over 100 stays, so the UI can show an over-limit state.
    if !raw.used_percent.is_finite() || raw.used_percent < 0.0 {
        return None;
    }
    let minutes = u32::try_from(raw.window_minutes).ok().filter(|m| *m > 0)?;
    let resets_at = match raw.resets_at {
        Some(secs) if secs.is_finite() => Some(DateTime::from_timestamp(secs.floor() as i64, 0)?),
        Some(_) => return None,
        None => None,
    };
    Some(Window::new(
        kind_from_minutes(minutes),
        raw.used_percent,
        resets_at,
    ))
}
