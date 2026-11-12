//! Pool totals, the rate history and the run-out forecast. All functions are pure.
//!
//! Times are epoch seconds. A percent is 0 to 100.

use std::collections::{HashMap, VecDeque};

use super::status::PoolStatus;

const WEEK_HOURS: f64 = 168.0;
const WEEK_SECS: i64 = 168 * 3600;
/// The forecast looks this far ahead.
pub const HORIZON_HOURS: f64 = 168.0;
/// The history keeps a sample this long, in seconds.
const KEEP_SECS: i64 = 90 * 60;
/// The recent rate uses the samples of this many seconds.
const RECENT_SECS: i64 = 60 * 60;
/// The recent rate needs the samples to span at least this many seconds.
const MIN_SPAN_SECS: i64 = 20 * 60;
/// Two resets this close count as the same reset.
const RESET_TOLERANCE_SECS: i64 = 60;
const MAX_STEPS: usize = 1000;

/// The basis of a rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    Recent,
    Week,
}

/// The totals of a pool.
#[derive(Debug, Clone, PartialEq)]
pub struct PoolTotal {
    pub five_hour_pct: Option<f64>,
    pub seven_day_pct: Option<f64>,
    pub open_seats: u32,
    pub seat_count: u32,
    pub next_refill_at: Option<i64>,
}

/// The forecast of a pool.
#[derive(Debug, Clone, PartialEq)]
pub struct Forecast {
    pub runs_out_at: Option<i64>,
    pub rate_pct_per_hour: f64,
    pub basis: Basis,
}

/// One seat in a sample: the week percent and the week reset.
#[derive(Debug, Clone, PartialEq)]
pub struct SeatSample {
    pub seat: String,
    pub week_pct: f64,
    pub reset: Option<i64>,
}

/// The week state of a pool at one `updated_epoch`.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub updated_epoch: i64,
    pub seats: Vec<SeatSample>,
}

impl Sample {
    pub fn from_status(status: &PoolStatus) -> Self {
        Self {
            updated_epoch: status.updated_epoch,
            seats: status
                .seats
                .iter()
                .filter_map(|s| {
                    Some(SeatSample {
                        seat: s.seat.clone(),
                        week_pct: s.week.used_pct?,
                        reset: s.week.reset_epoch,
                    })
                })
                .collect(),
        }
    }
}

/// The samples of every pool, in memory. Each pool keeps its samples for 90 minutes.
#[derive(Debug, Clone, Default)]
pub struct RateHistory {
    pools: HashMap<String, VecDeque<Sample>>,
}

impl RateHistory {
    /// Adds a sample when its `updated_epoch` is new, then drops the old samples.
    pub fn record(&mut self, pool: &str, status: &PoolStatus, now: i64) {
        let samples = self.pools.entry(pool.to_owned()).or_default();
        if samples
            .back()
            .is_none_or(|s| s.updated_epoch < status.updated_epoch)
        {
            samples.push_back(Sample::from_status(status));
        }
        while samples
            .front()
            .is_some_and(|s| s.updated_epoch < now - KEEP_SECS)
        {
            samples.pop_front();
        }
    }

    pub fn samples(&self, pool: &str) -> Vec<Sample> {
        self.pools
            .get(pool)
            .map(|d| d.iter().cloned().collect())
            .unwrap_or_default()
    }
}

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, n) = values.fold((0.0, 0u32), |(s, n), v| (s + v, n + 1));
    (n > 0).then(|| sum / f64::from(n))
}

/// Computes the totals of a pool at `now`.
pub fn pool_total(status: &PoolStatus, now: i64) -> PoolTotal {
    PoolTotal {
        five_hour_pct: mean(
            status
                .seats
                .iter()
                .filter_map(|s| s.five_hour.as_of(now).used_pct),
        ),
        seven_day_pct: mean(
            status
                .seats
                .iter()
                .filter_map(|s| s.week.as_of(now).used_pct),
        ),
        open_seats: status
            .seats
            .iter()
            .filter(|s| s.state.is_available())
            .count() as u32,
        seat_count: status.seats.len() as u32,
        next_refill_at: status
            .seats
            .iter()
            .filter_map(|s| s.week.reset_epoch)
            .filter(|r| *r > now)
            .min(),
    }
}

/// The week percent left per hour until the week reset of a seat. `None` when unknown.
pub fn seat_score(week_pct: Option<f64>, reset: Option<i64>, now: i64) -> Option<f64> {
    let hours = (reset? - now) as f64 / 3600.0;
    let pct = week_pct?;
    (hours > 0.0).then(|| (100.0 - pct).max(0.0) / hours)
}

/// The recent rate in pool percent per hour, or `None` when the samples are too few.
pub fn recent_rate(samples: &[Sample], now: i64) -> Option<f64> {
    let recent: Vec<&Sample> = samples
        .iter()
        .filter(|s| s.updated_epoch >= now - RECENT_SECS)
        .collect();
    let (first, last) = (recent.first()?, recent.last()?);
    let span = last.updated_epoch - first.updated_epoch;
    if span < MIN_SPAN_SECS || last.seats.is_empty() {
        return None;
    }
    let rise: f64 = last
        .seats
        .iter()
        .filter_map(|l| {
            let f = first.seats.iter().find(|f| f.seat == l.seat)?;
            let same_reset = match (f.reset, l.reset) {
                (Some(a), Some(b)) => (a - b).abs() <= RESET_TOLERANCE_SECS,
                (None, None) => true,
                _ => false,
            };
            same_reset.then(|| (l.week_pct - f.week_pct).max(0.0))
        })
        .sum();
    Some(rise / (span as f64 / 3600.0) / last.seats.len() as f64)
}

/// The week average rate in pool percent per hour.
pub fn week_rate(status: &PoolStatus, now: i64) -> f64 {
    if status.seats.is_empty() {
        return 0.0;
    }
    let sum: f64 = status
        .seats
        .iter()
        .filter_map(|s| {
            let reset = s.week.reset_epoch.filter(|r| *r > now)?;
            let elapsed = WEEK_HOURS - (reset - now) as f64 / 3600.0;
            (elapsed >= 1.0).then(|| s.week.used_pct.unwrap_or(0.0) / elapsed)
        })
        .sum();
    sum / status.seats.len() as f64
}

/// One seat in the run-out simulation: percent left and the next reset, in hours from now.
#[derive(Debug, Clone, Copy)]
struct SimSeat {
    left: f64,
    reset_h: Option<f64>,
}

fn sim_seats(status: &PoolStatus, now: i64) -> Vec<SimSeat> {
    status
        .seats
        .iter()
        .map(|s| {
            let week = s.week.as_of(now);
            let reset_h = week.reset_epoch.map(|r| {
                // A reset that passed rolls forward by whole weeks.
                let mut r = r;
                while r <= now {
                    r += WEEK_SECS;
                }
                (r - now) as f64 / 3600.0
            });
            SimSeat {
                left: (100.0 - week.used_pct.unwrap_or(0.0)).max(0.0),
                reset_h,
            }
        })
        .collect()
}

/// Simulates the pool from now. Returns the hours until it runs out, or `None` inside the horizon.
///
/// The demand is `rate * seat count` seat percent per hour. Seats drain soonest reset first.
/// A seat that resets refills to 100 and resets again 168 hours later.
fn simulate(seats: &mut [SimSeat], demand: f64) -> Option<f64> {
    if demand <= 0.0 || seats.is_empty() {
        return None;
    }
    let mut t = 0.0;
    for _ in 0..MAX_STEPS {
        let next = seats
            .iter()
            .enumerate()
            .filter_map(|(i, s)| Some((i, s.reset_h?)))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let total: f64 = seats.iter().map(|s| s.left).sum();
        let hit = t + total / demand;
        let limit = next.map_or(HORIZON_HOURS, |(_, r)| r.min(HORIZON_HOURS));
        if hit <= limit {
            return (hit <= HORIZON_HOURS).then_some(hit);
        }
        let (i, t_r) = next.filter(|(_, r)| *r <= HORIZON_HOURS)?;
        drain(seats, demand * (t_r - t));
        seats[i].left = 100.0;
        seats[i].reset_h = Some(t_r + WEEK_HOURS);
        t = t_r;
    }
    None
}

/// Takes `amount` seat percent from the seats, soonest reset first.
fn drain(seats: &mut [SimSeat], mut amount: f64) {
    let mut order: Vec<usize> = (0..seats.len()).collect();
    order.sort_by(|a, b| {
        let key = |i: usize| seats[i].reset_h.unwrap_or(f64::INFINITY);
        key(*a).total_cmp(&key(*b))
    });
    for i in order {
        let take = amount.min(seats[i].left);
        seats[i].left -= take;
        amount -= take;
    }
}

/// Builds the forecast of a pool at `now` from its samples and its latest status.
pub fn forecast(status: &PoolStatus, samples: &[Sample], now: i64) -> Option<Forecast> {
    if status.seats.is_empty() {
        return None;
    }
    let (rate, basis) = match recent_rate(samples, now) {
        Some(rate) => (rate, Basis::Recent),
        None => (week_rate(status, now), Basis::Week),
    };
    let mut seats = sim_seats(status, now);
    let runs_out_at = simulate(&mut seats, rate * status.seats.len() as f64)
        .map(|h| now + (h * 3600.0).round() as i64);
    Some(Forecast {
        runs_out_at,
        rate_pct_per_hour: rate,
        basis,
    })
}
