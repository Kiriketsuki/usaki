//! The hub data that the backend sends to the webview. Times are epoch milliseconds.
//!
//! The wire shape is in `docs/design/claude-hub.md`.

use serde::Serialize;
use usaki_core::providers::claude_hub::forecast::seat_score;
use usaki_core::providers::claude_hub::{Basis, DataSource, PoolReading, SeatStatus};

use crate::view::WindowView;
use usaki_core::model::{FIVE_HOUR_MINUTES, SEVEN_DAY_MINUTES};

/// The mode of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UsakiMode {
    Normal,
    Hub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceView {
    Cache,
    Endpoint,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeatView {
    pub name: String,
    pub priority: Option<i64>,
    pub state: &'static str,
    pub unavailable: bool,
    pub next: bool,
    pub extra_usage_open: bool,
    pub spend_guard_blocked: bool,
    /// `None` when the live folder does not exist.
    pub live_sessions: Option<u32>,
    /// The week percent left per hour until the week reset.
    pub score: Option<f64>,
    pub five_hour: Option<WindowView>,
    pub seven_day: Option<WindowView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolTotalView {
    pub five_hour_pct: Option<f64>,
    pub seven_day_pct: Option<f64>,
    pub open_seats: u32,
    pub seat_count: u32,
    pub next_refill_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BasisView {
    Recent,
    Week,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastView {
    pub runs_out_at: Option<i64>,
    pub rate_pct_per_hour: f64,
    pub basis: BasisView,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolView {
    pub name: String,
    pub source: SourceView,
    pub updated_at: i64,
    pub fetched_at: i64,
    pub stale: bool,
    pub error: Option<String>,
    pub next_seat: Option<String>,
    pub extra_usage_open: Vec<String>,
    pub spend_guard_blocked: Vec<String>,
    pub seats: Vec<SeatView>,
    pub total: PoolTotalView,
    pub forecast: Option<ForecastView>,
}

fn millis(epoch: i64) -> i64 {
    epoch * 1000
}

fn window(
    w: usaki_core::providers::claude_hub::WindowStatus,
    minutes: u32,
    now: i64,
) -> Option<WindowView> {
    let w = w.as_of(now);
    Some(WindowView {
        minutes,
        used_pct: w.used_pct?,
        resets_at: w.reset_epoch.map(millis),
    })
}

fn seat_view(s: &SeatStatus, r: &PoolReading, now: i64) -> SeatView {
    let status = r.status.as_ref();
    let listed = |list: Option<&Vec<String>>| list.is_some_and(|l| l.contains(&s.seat));
    SeatView {
        name: s.seat.clone(),
        priority: s.priority,
        state: s.state.as_str(),
        unavailable: s.unavailable,
        next: status.and_then(|p| p.next_seat.as_deref()) == Some(s.seat.as_str()),
        extra_usage_open: listed(status.map(|p| &p.extra_usage_open)),
        spend_guard_blocked: listed(status.map(|p| &p.spend_guard_blocked)),
        live_sessions: r
            .live
            .as_ref()
            .map(|m| m.get(&s.seat).copied().unwrap_or(0)),
        score: seat_score(s.week.as_of(now).used_pct, s.week.reset_epoch, now),
        five_hour: window(s.five_hour, FIVE_HOUR_MINUTES, now),
        seven_day: window(s.week, SEVEN_DAY_MINUTES, now),
    }
}

impl PoolView {
    /// Builds the view of a pool reading at `now` (epoch seconds).
    pub fn from_reading(r: &PoolReading, now: i64) -> Self {
        let status = r.status.as_ref();
        Self {
            name: r.name.clone(),
            source: match r.source {
                DataSource::Cache => SourceView::Cache,
                DataSource::Endpoint => SourceView::Endpoint,
            },
            updated_at: millis(r.updated_epoch),
            fetched_at: millis(r.fetched_at),
            stale: r.stale,
            error: r.error.clone(),
            next_seat: status.and_then(|p| p.next_seat.clone()),
            extra_usage_open: status
                .map(|p| p.extra_usage_open.clone())
                .unwrap_or_default(),
            spend_guard_blocked: status
                .map(|p| p.spend_guard_blocked.clone())
                .unwrap_or_default(),
            seats: status
                .map(|p| p.seats.iter().map(|s| seat_view(s, r, now)).collect())
                .unwrap_or_default(),
            total: r.total.as_ref().map_or(
                PoolTotalView {
                    five_hour_pct: None,
                    seven_day_pct: None,
                    open_seats: 0,
                    seat_count: 0,
                    next_refill_at: None,
                },
                |t| PoolTotalView {
                    five_hour_pct: t.five_hour_pct,
                    seven_day_pct: t.seven_day_pct,
                    open_seats: t.open_seats,
                    seat_count: t.seat_count,
                    next_refill_at: t.next_refill_at.map(millis),
                },
            ),
            forecast: r.forecast.as_ref().map(|f| ForecastView {
                runs_out_at: f.runs_out_at.map(millis),
                rate_pct_per_hour: f.rate_pct_per_hour,
                basis: match f.basis {
                    Basis::Recent => BasisView::Recent,
                    Basis::Week => BasisView::Week,
                },
            }),
        }
    }
}
