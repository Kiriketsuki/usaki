//! The usage data that the backend sends to the webview.
//!
//! Times are epoch milliseconds, so the page can use them with `Date` directly.

use chrono::{DateTime, Utc};
use serde::Serialize;
use usaki_core::model::{FIVE_HOUR_MINUTES, SEVEN_DAY_MINUTES};
use usaki_core::{Snapshot, Window, WindowKind};

/// One usage window of an account.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowView {
    /// The window length. 300 is the 5h window and 10080 is the 7d window.
    pub minutes: u32,
    pub used_pct: f64,
    pub resets_at: Option<i64>,
}

/// One account as the bubble shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub id: String,
    pub stale: bool,
    pub fetched_at: i64,
    pub windows: Vec<WindowView>,
    /// The read error, when the provider could not read the account. The account is then stale.
    pub error: Option<String>,
}

/// The payload of the `usage://update` event and of the `get_usage` command.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageUpdate {
    pub polled_at: i64,
    pub accounts: Vec<AccountView>,
    /// A problem that stopped the discovery of accounts.
    pub error: Option<String>,
}

fn minutes(kind: WindowKind) -> u32 {
    match kind {
        WindowKind::FiveHour => FIVE_HOUR_MINUTES,
        WindowKind::SevenDay => SEVEN_DAY_MINUTES,
        WindowKind::Other(m) => m,
    }
}

fn millis(t: DateTime<Utc>) -> i64 {
    t.timestamp_millis()
}

impl From<&Window> for WindowView {
    fn from(w: &Window) -> Self {
        Self {
            minutes: minutes(w.kind),
            used_pct: w.used_pct,
            resets_at: w.resets_at.map(millis),
        }
    }
}

impl AccountView {
    /// Returns the view of a provider result. A failed read gives a stale account with no windows.
    pub fn from_result(id: &str, result: usaki_core::Result<Snapshot>, now: DateTime<Utc>) -> Self {
        match result {
            Ok(snap) => Self {
                id: snap.account_id.clone(),
                stale: snap.stale,
                fetched_at: millis(snap.fetched_at),
                windows: snap.windows.iter().map(WindowView::from).collect(),
                error: None,
            },
            Err(e) => Self {
                id: id.to_owned(),
                stale: true,
                fetched_at: millis(now),
                windows: Vec::new(),
                error: Some(e.to_string()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use usaki_core::{Error, Source};

    fn at(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(secs, 0).unwrap()
    }

    #[test]
    fn snapshot_maps_to_view() {
        let snap = Snapshot::new(
            "acc1",
            vec![
                Window::new(WindowKind::FiveHour, 42.0, Some(at(2_000))),
                Window::new(WindowKind::SevenDay, 17.5, None),
                Window::new(WindowKind::Other(60), 3.0, None),
            ],
            at(1_000),
            Source::ClaudeCache,
            false,
        );
        let view = AccountView::from_result("acc1", Ok(snap), at(1_060));
        assert_eq!(view.id, "acc1");
        assert!(!view.stale);
        assert_eq!(view.fetched_at, 1_000_000);
        assert_eq!(view.error, None);
        let mins: Vec<u32> = view.windows.iter().map(|w| w.minutes).collect();
        assert_eq!(mins, [300, 10080, 60]);
        assert_eq!(view.windows[0].resets_at, Some(2_000_000));
        assert_eq!(view.windows[1].used_pct, 17.5);
    }

    #[test]
    fn error_maps_to_stale_view() {
        let err = Error::Malformed {
            path: "/x/claude-acc2.json".into(),
            reason: "bad".into(),
        };
        let view = AccountView::from_result("acc2", Err(err), at(1_060));
        assert!(view.stale);
        assert!(view.windows.is_empty());
        assert_eq!(view.fetched_at, 1_060_000);
        assert!(view.error.unwrap().contains("not valid JSON"));
    }

    #[test]
    fn serializes_in_camel_case() {
        let view = WindowView {
            minutes: 300,
            used_pct: 1.0,
            resets_at: None,
        };
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"minutes": 300, "usedPct": 1.0, "resetsAt": null})
        );
    }
}
