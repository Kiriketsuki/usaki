//! Parses the reply of the usage API. Every function here is pure.

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::model::{FIVE_HOUR_MINUTES, SEVEN_DAY_MINUTES, Window, kind_from_minutes};

/// The longest error message that the provider keeps from an error body.
const MAX_MESSAGE_CHARS: usize = 200;

/// What one usage call means for the account.
#[derive(Debug, Clone, PartialEq)]
pub enum UsageReply {
    /// HTTP 200 with a body that parses.
    Usage(Vec<Window>),
    /// HTTP 401. The token is not valid.
    Unauthorized(Option<String>),
    /// HTTP 429. The provider must call less often.
    RateLimited(Option<String>),
    /// Any other status.
    Failed {
        status: u16,
        message: Option<String>,
    },
    /// HTTP 200 with a body that does not parse.
    BadBody(String),
}

/// Maps the status and body of a usage call to a `UsageReply`.
pub fn classify_reply(status: u16, body: &str) -> UsageReply {
    match status {
        200 => match parse_usage_body(body) {
            Ok(windows) => UsageReply::Usage(windows),
            Err(reason) => UsageReply::BadBody(reason),
        },
        401 => UsageReply::Unauthorized(error_message(body)),
        429 => UsageReply::RateLimited(error_message(body)),
        _ => UsageReply::Failed {
            status,
            message: error_message(body),
        },
    }
}

/// Parses a 200 body into windows.
///
/// The body has `five_hour` and `seven_day` objects. Each has `utilization`, a percent from
/// 0 to 100, and `resets_at`, an RFC 3339 string. A missing or null window gives no window.
/// The parser ignores every other key.
pub fn parse_usage_body(body: &str) -> Result<Vec<Window>, String> {
    let root: Value = serde_json::from_str(body)
        .map_err(|e| format!("The usage reply is not valid JSON. {e}"))?;
    let obj = root
        .as_object()
        .ok_or("The usage reply must be a JSON object.")?;
    let mut out = Vec::new();
    for (key, minutes) in [
        ("five_hour", FIVE_HOUR_MINUTES),
        ("seven_day", SEVEN_DAY_MINUTES),
    ] {
        match obj.get(key) {
            None | Some(Value::Null) => {}
            Some(v) => {
                if let Some(w) = parse_window(key, minutes, v)? {
                    out.push(w);
                }
            }
        }
    }
    Ok(out)
}

fn parse_window(key: &str, minutes: u32, v: &Value) -> Result<Option<Window>, String> {
    let obj = v
        .as_object()
        .ok_or_else(|| format!("The field {key} must be an object."))?;
    let used = match obj.get("utilization") {
        None | Some(Value::Null) => return Ok(None),
        Some(n) => n
            .as_f64()
            .filter(|n| n.is_finite())
            .ok_or_else(|| format!("The field {key}.utilization must be a number."))?,
    };
    // A value over 100 stays, so the UI can show an over-limit state.
    if used < 0.0 {
        return Err(format!("The field {key}.utilization must not be negative."));
    }
    let resets_at = match obj.get("resets_at") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(
            DateTime::parse_from_rfc3339(s)
                .map(|d| d.with_timezone(&Utc))
                .map_err(|_| format!("The field {key}.resets_at must be an RFC 3339 string."))?,
        ),
        Some(_) => {
            return Err(format!(
                "The field {key}.resets_at must be an RFC 3339 string."
            ));
        }
    };
    Ok(Some(Window::new(
        kind_from_minutes(minutes),
        used,
        resets_at,
    )))
}

/// Returns the message of an error body, or its type when it has no message.
///
/// The API sends `{"error": {"type": ..., "message": ...}}`. The function cuts a long message.
pub fn error_message(body: &str) -> Option<String> {
    let root: Value = serde_json::from_str(body).ok()?;
    let err = root.get("error")?;
    let text = match err {
        Value::String(s) => s.as_str(),
        Value::Object(o) => o
            .get("message")
            .and_then(Value::as_str)
            .or_else(|| o.get("type").and_then(Value::as_str))?,
        _ => return None,
    };
    let text = text.trim();
    (!text.is_empty()).then(|| text.chars().take(MAX_MESSAGE_CHARS).collect())
}
