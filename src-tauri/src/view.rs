//! The usage data that the backend sends to the webview.
//!
//! Times are epoch milliseconds, so the page can use them with `Date` directly.

use chrono::{DateTime, Utc};
use serde::Serialize;
use usaki_core::model::{FIVE_HOUR_MINUTES, SEVEN_DAY_MINUTES};
use usaki_core::{Credits, Snapshot, Window, WindowKind};

/// The provider of an account. The wire name is the first part of the account id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderName {
    Claude,
    Codex,
}

impl ProviderName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

/// Returns the wire id of an account: `<provider>:<name>`. A Claude account and a Codex
/// account with the same name get two ids, so they never merge.
pub fn wire_id(provider: ProviderName, name: &str) -> String {
    format!("{}:{name}", provider.as_str())
}

/// The credits state of an account. The balance is the string that the provider gives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditsView {
    pub has_credits: bool,
    pub unlimited: bool,
    pub balance: Option<String>,
}

impl From<&Credits> for CreditsView {
    fn from(c: &Credits) -> Self {
        Self {
            has_credits: c.has_credits,
            unlimited: c.unlimited,
            balance: c.balance.clone(),
        }
    }
}

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
    /// The wire id, `<provider>:<name>`. The page keys pins by it.
    pub id: String,
    pub provider: ProviderName,
    /// The account name that the cell shows.
    pub name: String,
    pub stale: bool,
    pub fetched_at: i64,
    pub windows: Vec<WindowView>,
    /// The read error or the stale reason. A read error makes the account stale.
    pub error: Option<String>,
    /// The credits state, for a provider that reports one.
    pub credits: Option<CreditsView>,
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
    /// The stale reason of a snapshot goes into `error`.
    pub fn from_result(
        provider: ProviderName,
        name: &str,
        result: usaki_core::Result<Snapshot>,
        now: DateTime<Utc>,
    ) -> Self {
        let base = Self {
            id: wire_id(provider, name),
            provider,
            name: name.to_owned(),
            stale: true,
            fetched_at: millis(now),
            windows: Vec::new(),
            error: None,
            credits: None,
        };
        match result {
            Ok(snap) => Self {
                stale: snap.stale,
                fetched_at: millis(snap.fetched_at),
                windows: snap.windows.iter().map(WindowView::from).collect(),
                error: snap.stale_reason.clone(),
                credits: snap.credits.as_ref().map(CreditsView::from),
                ..base
            },
            Err(e) => Self {
                error: Some(e.to_string()),
                ..base
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
        let view = AccountView::from_result(ProviderName::Claude, "acc1", Ok(snap), at(1_060));
        assert_eq!(view.id, "claude:acc1");
        assert_eq!(view.name, "acc1");
        assert_eq!(view.credits, None);
        assert!(!view.stale);
        assert_eq!(view.fetched_at, 1_000_000);
        assert_eq!(view.error, None);
        let mins: Vec<u32> = view.windows.iter().map(|w| w.minutes).collect();
        assert_eq!(mins, [300, 10080, 60]);
        assert_eq!(view.windows[0].resets_at, Some(2_000_000));
        assert_eq!(view.windows[1].used_pct, 17.5);
    }

    #[test]
    fn stale_reason_maps_to_error() {
        let snap = Snapshot::new("acc1", Vec::new(), at(1_000), Source::ClaudeWatch, true)
            .with_stale_reason("The usage API rejected the token (HTTP 401).");
        let view = AccountView::from_result(ProviderName::Claude, "acc1", Ok(snap), at(1_060));
        assert!(view.stale);
        assert!(view.error.unwrap().contains("401"));
    }

    #[test]
    fn error_maps_to_stale_view() {
        let err = Error::Malformed {
            path: "/x/claude-acc2.json".into(),
            reason: "bad".into(),
        };
        let view = AccountView::from_result(ProviderName::Claude, "acc2", Err(err), at(1_060));
        assert_eq!(view.id, "claude:acc2");
        assert!(view.stale);
        assert!(view.windows.is_empty());
        assert_eq!(view.fetched_at, 1_060_000);
        assert!(view.error.unwrap().contains("not valid JSON"));
    }

    #[test]
    fn codex_view_carries_the_provider_and_the_credits() {
        let credits = Credits {
            has_credits: true,
            unlimited: false,
            balance: Some("12.50".into()),
        };
        let snap = Snapshot::new(
            "codex",
            vec![Window::new(WindowKind::SevenDay, 37.0, None)],
            at(1_000),
            Source::CodexLogs,
            false,
        )
        .with_credits(Some(credits));
        let view = AccountView::from_result(ProviderName::Codex, "codex", Ok(snap), at(1_060));
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["id"], "codex:codex");
        assert_eq!(json["provider"], "codex");
        assert_eq!(json["name"], "codex");
        assert_eq!(
            json["credits"],
            serde_json::json!({"hasCredits": true, "unlimited": false, "balance": "12.50"})
        );
    }

    #[test]
    fn same_name_gets_two_wire_ids() {
        assert_ne!(
            wire_id(ProviderName::Claude, "main"),
            wire_id(ProviderName::Codex, "main")
        );
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
