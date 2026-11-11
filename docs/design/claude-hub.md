# Claude hub mode

Hub mode shows the pools of a Claude hub. A pool is a group of seats. A seat is one Claude
account behind the hub. The names in this file are synthetic: pools `pool-a` and `pool-b`, seats
`acc1` to `acc4`.

## Modes

- `normal`: the bubble works as before. `mode` is `normal` and `pools` is empty. Seat cache
  files `claude-<seat>.json` stay direct accounts.
- `hub`: `mode` is `hub` and `pools` holds the pools. A direct Claude account whose name is a seat of a
  pool with data leaves `accounts`. Codex accounts and other Claude accounts stay.
- `auto`: the app is in hub mode when at least one pool exists.

The setting comes from the first of these:

1. The `USAKI_MODE` environment variable (`auto`, `normal` or `hub`).
2. The top-level `mode = "..."` key in `config.toml` in `dirs::config_dir()/usaki/`.
   `USAKI_CONFIG_DIR` replaces that folder, for tests.
3. `auto`.

A bad value falls back to `auto`.

## Pool discovery

The config folder is `$CLIPROXY_CONF_DIR`, else `~/.config/cliproxy`. The cache folder is
`$CLIPROXY_CACHE_DIR`, else `<cache base>/cliproxy`. A pool exists when one of these is true:

- `<cache>/<pool>.status.json` exists.
- Both `<conf>/<pool>.status-url` and `<conf>/<pool>.key` exist.

A pool name uses only `[a-z0-9-]`. usaKi skips any other name. A pool that exists but has no
readable data still shows. It is stale, has an error and has no seats.

## Status JSON

```json
{ "pool": "pool-a", "updated_epoch": 1800000000, "next_seat": "acc1",
  "extra_usage_open": ["acc3"], "spend_guard_blocked": [],
  "fetched_at": 1800000005,
  "seats": [ { "seat": "acc1", "priority": 100, "state": "open",
    "week_used": 0.51, "week_hours_left": 116.3, "week_reset_epoch": 1800418680,
    "five_hour_used": 0.06, "five_hour_hours_left": 2.5, "five_hour_reset_epoch": 1800009000,
    "unavailable": false } ] }
```

- `*_used` is a fraction from 0 to 1. It can be null. The percent is the fraction times 100.
- The reset is `*_reset_epoch` when present. Else it is `updated_epoch + hours_left * 3600`.
  Else there is no reset.
- `state` is `open`, `warm`, `five_hour_full` or `week_full`. Any other value is `unknown`.
- `fetched_at` is optional. The parser ignores extra keys. `seats` must be an array, else the parser
  rejects the file.
- A window whose reset has passed counts as 0%.

## Source rule

1. Read `<cache>/<pool>.status.json`. The age is `now - updated_epoch`.
2. If the age is 120 s or less, use it. The source is `cache`.
3. Else, if the endpoint config exists, call the endpoint. The call happens at most once per 60 s
   for each pool. The source is then `endpoint`, and usaKi keeps the reply in memory.
4. Else use the newest data that it has: the cache file or the last endpoint reply.
5. The data is stale when its age is over 600 s.

usaKi never writes the cliproxy cache.

### Why the cache comes first

While a hub session runs, the statusline refreshes the cache file each minute. A cache read needs
no network and no key, so usaKi reads it first. When no hub session is open, the cache goes
stale. usaKi must still show the pools, so it then calls the endpoint as a fallback.

### The endpoint call

- URL: the first line of `<conf>/<pool>.status-url`.
- Header `X-Api-Key`: the first line of `<conf>/<pool>.key`, trimmed.
- usaKi makes the call only when the URL is https and its host equals the host of
  `<conf>/<pool>.url`. Else it refuses the call and shows an error.
- No redirects. Timeout 5 s. The body limit is 1 MB.
- The key never appears in a log, an error or a test output.
- usaKi replaces the key with `[key]` in any error text before it keeps the text.

## Live sessions

The optional files `<cache>/<pool>.live/*.json` each hold `{ "seat": "acc1", "written_at": <epoch s> }`.
A file is live when `now - written_at` is 600 s or less. `liveSessions` of a seat is the count of
live files for it. When the `.live` folder does not exist, `liveSessions` is null for each seat,
which means unknown. A statusline follow-up outside this repo writes these files.

## Forecast

- Pool 7d percent is the mean of the seat week percents. Pool 5h percent is the mean of the seat
  5h percents. The mean leaves out a seat with no value.
- Rate (pool percent per hour). usaKi keeps one sample for each new `updated_epoch` of a pool, for
  90 minutes. The recent rate needs samples in the last 60 minutes that span at least 20 minutes.
  The poller then adds the rise of the week percent of each seat. It divides the sum by the hours and
  by the seat count. It leaves out a seat whose week reset moved by more than 60 s. The basis is
  `recent`.
- Else the rate is the week average. For each seat, it is `pct / elapsed hours`, with
  `elapsed = 168 - hours to reset`. usaKi leaves out a seat with less than 1 h elapsed. It
  divides the sum by the seat count. The basis is `week`.
- Run out: usaKi simulates 168 h from now. The demand is `rate * seat count` seat percent per
  hour. Seats drain soonest reset first. A seat that resets refills to 100 and resets again 168 h
  later. If the pool runs out inside the horizon, `runsOutAt` is that time. Else, or when the rate
  is 0, it is null.

The score of a seat is its week percent left per hour until its week reset.

## Wire shape

The backend sends `UsageUpdate` with camelCase keys and epoch milliseconds. The TypeScript types
are in `ui/src/usage.ts`. The Rust types are in `src-tauri/src/hub_view.rs`.

```ts
export type UsakiMode = 'normal' | 'hub';
export type SeatState = 'open' | 'warm' | 'five_hour_full' | 'week_full' | 'unknown';
export interface SeatView {
  name: string; priority: number | null; state: SeatState; unavailable: boolean;
  next: boolean; extraUsageOpen: boolean; spendGuardBlocked: boolean;
  liveSessions: number | null; score: number | null;
  fiveHour: WindowView | null; sevenDay: WindowView | null;
}
export interface PoolTotal {
  fiveHourPct: number | null; sevenDayPct: number | null;
  openSeats: number; seatCount: number; nextRefillAt: number | null;
}
export interface PoolForecast { runsOutAt: number | null; ratePctPerHour: number; basis: 'recent' | 'week'; }
export interface PoolView {
  name: string; source: 'cache' | 'endpoint'; updatedAt: number; fetchedAt: number;
  stale: boolean; error: string | null; nextSeat: string | null;
  extraUsageOpen: string[]; spendGuardBlocked: string[];
  seats: SeatView[]; total: PoolTotal; forecast: PoolForecast | null;
}
export interface UsageUpdate { polledAt: number; accounts: AccountView[]; error: string | null;
  mode: UsakiMode; pools: PoolView[]; }
```

`total` of a pool with no data has zero counts and null values.

## Code

- `crates/usaki-core/src/providers/claude_hub/`: parse, discovery, source rule, live sessions
  and forecast.
- `src-tauri/src/hub.rs`: mode resolution and the state that the poller keeps between polls.
- `src-tauri/src/hub_view.rs`: the wire types.
