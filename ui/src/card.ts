// The expanded card: one row per account with both windows. A port of
// docs/design/mockup/Card.dc.html. It holds no DOM code.

import { type Head, type Pin, pickHead } from './centre';
import { BLONDE, DEFAULT_THRESHOLDS, EMERALD, RUBY, type Thresholds, tone } from './colour';
import { ageText, clockAt, minutesUntil, spacedLeft } from './countdown';
import { poolAccount, poolForId, viewAccounts } from './hub';
import { PROVIDER_COLOUR } from './marks';
import { type MoonView, NEW_MOON, moonView } from './moon';
import {
  type AccountView,
  type Credits,
  FIVE_HOUR_MINUTES,
  SEVEN_DAY_MINUTES,
  type PoolView,
  type ProviderName,
  type SeatView,
  type UsageUpdate,
  type WindowView,
  noData,
  weekOnly,
  windowAt
} from './usage';

/** The card width from the mockup, in px. */
export const CARD_WIDTH = 420;
/** The tri-primary bar on top of the card, in px. */
export const CARD_BAR = 3;
/** The header: 44 px buttons, 4 px padding above and below, and a 1 px rule. */
export const CARD_HEADER = 53;
/** The height of one line under the title of a block: a window row or the credits row. */
export const CARD_LINE = 16;
/** The gap between two lines of a block. */
export const CARD_LINE_GAP = 6;
/** A block without its lines: 12 px padding above and below, a 16 px title, an 8 px gap and a 1 px rule. */
export const CARD_BLOCK = 49;
/** The block of a normal account, with two window rows. */
export const CARD_ROW = blockHeight(2);

/** Returns the height of an account block with `lines` lines under its title. */
export function blockHeight(lines: number): number {
  return CARD_BLOCK + lines * CARD_LINE + Math.max(0, lines - 1) * CARD_LINE_GAP;
}

/** The colour of a bar with no data. */
const NO_BAR = '#363a4f';

/** One window row: the label, the bar, the percent, the moon and the reset. */
export interface WindowRow {
  label: '5h' | '7d';
  /** The bar fill in percent, 0..100. */
  bar: number;
  pct: string;
  color: string;
  /** The countdown, `ready` for a window with no countdown, or `--` for a missing window. */
  left: string;
  /** The clock time of the reset, or '' when no countdown runs. */
  at: string;
  /** The moon of a 7d window. A 5h row and a missing 7d window have none. */
  moon: MoonView | null;
}

/** One account block. */
export interface AccountRow {
  id: string;
  /** The account name that the title shows. */
  name: string;
  /** The provider. The title shows its mark, the same polygon as the cell mark. */
  provider: ProviderName;
  /** The colour of the mark: the provider accent from PROVIDER_COLOUR. */
  colour: string;
  /** `next 5h` or `next 7d` on the auto head, `pinned` on a pinned head, else null. */
  tag: string | null;
  /** `stale 3h` for a stale account, else null. The rows then dim. */
  stale: string | null;
  /** The 5h row and the 7d row. A 7d-only account has the 7d row only. */
  windows: WindowRow[];
  /** True for an account with no 5h and no 7d window. The block then shows one `no data` line. */
  noData: boolean;
  /** The credits balance as the provider gives it, `unlimited`, or null for no credits row. */
  credits: string | null;
  /** The block height in px. */
  height: number;
  /** The pool block of a hub pool cell. It replaces the account lines. Absent for an account. */
  pool?: PoolBlock;
}

/** One seat line of a pool block. */
export interface SeatRow {
  name: string;
  /** True for the next seat of the pool. */
  next: boolean;
  state: string;
  stateColour: string;
  /** `p100`, or `p--` for no priority. */
  priority: string;
  five: { pct: string; left: string };
  seven: { pct: string; left: string };
  /** `3 live`, or null when the count is unknown. */
  live: string | null;
  /** True for an unavailable seat. The line then dims. */
  dim: boolean;
}

/** The pool block of a card row. The 5h and 7d rows reuse WindowRow. */
export interface PoolBlock {
  /** `next acc1`, or null when the pool has no next seat. */
  next: string | null;
  /** The 5h row and the 7d row of the pool total. */
  windows: WindowRow[];
  /** `runs out in 1d 4h, 0.6%/h, last 1h`, the pool error, or null. */
  forecast: string | null;
  /** `refill in 2d 3h`, or null. */
  refill: string | null;
  /** `extra usage: acc3` and `spend guard: acc2`, one line each when its list is not empty. */
  warnings: string[];
  seats: SeatRow[];
}

export interface CardView {
  /** `updated 2m ago`, from the newest data of any account. */
  updated: string;
  rows: AccountRow[];
  width: number;
  height: number;
}

/**
 * Returns the card size for blocks of the given heights. Zero blocks still get one normal
 * block, for the no data text.
 */
export function cardSize(blocks: readonly number[]): { width: number; height: number } {
  const body = blocks.length ? blocks.reduce((sum, h) => sum + h, 0) : CARD_ROW;
  return { width: CARD_WIDTH, height: CARD_BAR + CARD_HEADER + body };
}

/** Returns the row of the window of `minutes` length of `account` at `now`. */
export function windowRow(
  account: AccountView,
  minutes: number,
  now: number,
  thresholds: Thresholds = DEFAULT_THRESHOLDS,
  timeZone?: string
): WindowRow {
  const week = minutes === SEVEN_DAY_MINUTES;
  const label: WindowRow['label'] = week ? '7d' : '5h';
  const w = windowAt(account, minutes, now);
  if (!w) return { label, bar: 0, pct: '--', color: NO_BAR, left: '--', at: '', moon: null };
  const pct = Math.round(w.pct);
  const base = { label, bar: Math.max(0, Math.min(100, w.pct)), pct: pct + '%', color: tone(w.pct, thresholds) };
  if (w.ready || w.resetsAt == null) {
    return { ...base, left: 'ready', at: '', moon: week ? NEW_MOON : null };
  }
  const minsLeft = minutesUntil(w.resetsAt, now);
  const left = spacedLeft(minsLeft);
  return { ...base, left, at: clockAt(w.resetsAt, week, timeZone), moon: week ? moonView(minsLeft, left) : null };
}

/** Returns `stale` plus the age of the data, or null for a live account. */
export function staleText(account: AccountView, now: number): string | null {
  if (!account.stale) return null;
  const age = ageText(now - account.fetchedAt);
  return age ? 'stale ' + age : 'stale';
}

function headTag(head: Head): string | null {
  if (head.mode === 'auto' && head.account) {
    if (noData(head.account)) return null;
    return weekOnly(head.account) ? 'next 7d' : 'next 5h';
  }
  if (head.mode === 'pinned') return 'pinned';
  return null;
}

const STATE_COLOUR: Record<string, string> = {
  open: EMERALD,
  warm: BLONDE,
  five_hour_full: RUBY,
  week_full: RUBY
};
const STATE_UNKNOWN = '#a0a4b8';

function seatWindow(w: WindowView | null, now: number): { pct: string; left: string } {
  if (!w) return { pct: '--', left: '--' };
  const ready = w.resetsAt == null || w.resetsAt <= now;
  return {
    pct: Math.round(ready ? 0 : w.usedPct) + '%',
    left: ready || w.resetsAt == null ? 'ready' : spacedLeft(minutesUntil(w.resetsAt, now))
  };
}

function seatRow(s: SeatView, now: number): SeatRow {
  return {
    name: s.name,
    next: s.next,
    state: s.state.replace(/_/g, ' '),
    stateColour: STATE_COLOUR[s.state] ?? STATE_UNKNOWN,
    priority: 'p' + (s.priority ?? '--'),
    five: seatWindow(s.fiveHour, now),
    seven: seatWindow(s.sevenDay, now),
    live: s.liveSessions == null ? null : s.liveSessions + ' live',
    dim: s.unavailable
  };
}

/** Returns the forecast line of a pool, the pool error when it has no forecast, or null. */
export function forecastText(pool: PoolView, now: number): string | null {
  const f = pool.forecast;
  if (!f) return pool.error;
  const when =
    f.runsOutAt == null
      ? 'lasts past 7d'
      : f.runsOutAt <= now
        ? 'runs out now'
        : 'runs out in ' + spacedLeft(minutesUntil(f.runsOutAt, now));
  return [when, f.ratePctPerHour.toFixed(1) + '%/h', f.basis === 'recent' ? 'last 1h' : 'week avg'].join(', ');
}

/** Returns the pool block of `pool` at `now`. */
export function poolBlock(
  pool: PoolView,
  now: number,
  thresholds: Thresholds = DEFAULT_THRESHOLDS,
  timeZone?: string
): PoolBlock {
  const cell = poolAccount(pool, now);
  const refillAt = pool.total.nextRefillAt;
  const warnings = [
    pool.extraUsageOpen.length ? 'extra usage: ' + pool.extraUsageOpen.join(', ') : null,
    pool.spendGuardBlocked.length ? 'spend guard: ' + pool.spendGuardBlocked.join(', ') : null
  ].filter((w): w is string => w != null);
  return {
    next: pool.nextSeat ? 'next ' + pool.nextSeat : null,
    windows: [FIVE_HOUR_MINUTES, SEVEN_DAY_MINUTES].map((m) => windowRow(cell, m, now, thresholds, timeZone)),
    forecast: forecastText(pool, now),
    refill: refillAt != null && refillAt > now ? 'refill in ' + spacedLeft(minutesUntil(refillAt, now)) : null,
    warnings,
    seats: pool.seats.map((s) => seatRow(s, now))
  };
}

/** Returns the line count under the title of a pool block. */
export function poolLines(b: PoolBlock): number {
  return b.windows.length + (b.forecast ? 1 : 0) + (b.refill ? 1 : 0) + b.warnings.length + b.seats.length;
}

const BALANCE_FORMAT = new Intl.NumberFormat('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

/**
 * Returns the text of the credits row, or null when the account reports no credits. The
 * balance shows with 2 decimals and thousands separators, such as `21,690.58`. The logs do
 * not name the unit, so the text adds none. A balance that is not a number shows as given.
 */
export function creditsText(credits: Credits | null): string | null {
  if (!credits) return null;
  if (credits.unlimited) return 'unlimited';
  const balance = credits.balance?.trim();
  if (!balance) return null;
  const n = Number(balance);
  return Number.isFinite(n) ? BALANCE_FORMAT.format(n) : balance;
}

/** Returns `updated 2m ago` from the newest data of any account that was read. */
export function updatedText(accounts: readonly AccountView[], now: number): string {
  const read = accounts.filter((a) => a.error == null).map((a) => a.fetchedAt);
  if (read.length === 0) return 'no data';
  const age = ageText(now - Math.max(...read));
  return age ? 'updated ' + age + ' ago' : 'updated now';
}

/**
 * Returns the card at `now`. The rows keep the account order, so a row does not jump when
 * the head changes. The head row carries a tag that says how the head was chosen.
 */
export function cardView(
  update: UsageUpdate | null,
  now: number,
  pin: Pin,
  thresholds: Thresholds = DEFAULT_THRESHOLDS,
  timeZone?: string
): CardView {
  const accounts = viewAccounts(update);
  const head = pickHead(accounts, pin, now);
  const rows = accounts.map((a): AccountRow => {
    const pool = poolForId(update, a.id);
    if (pool) {
      const block = poolBlock(pool, now, thresholds, timeZone);
      return {
        id: a.id,
        name: a.name,
        provider: a.provider,
        colour: PROVIDER_COLOUR[a.provider],
        tag: block.next,
        stale: staleText(a, now),
        windows: block.windows,
        noData: false,
        credits: null,
        height: blockHeight(poolLines(block)),
        pool: block
      };
    }
    const empty = noData(a);
    const minutes = empty ? [] : weekOnly(a) ? [SEVEN_DAY_MINUTES] : [FIVE_HOUR_MINUTES, SEVEN_DAY_MINUTES];
    const windows = minutes.map((m) => windowRow(a, m, now, thresholds, timeZone));
    const credits = creditsText(a.credits);
    // An account with no window has one `no data` line in place of the window rows.
    const lines = (empty ? 1 : windows.length) + (credits == null ? 0 : 1);
    return {
      id: a.id,
      name: a.name,
      provider: a.provider,
      colour: PROVIDER_COLOUR[a.provider],
      tag: a === head.account ? headTag(head) : null,
      stale: staleText(a, now),
      windows,
      noData: empty,
      credits,
      height: blockHeight(lines)
    };
  });
  return { updated: updatedText(accounts, now), rows, ...cardSize(rows.map((r) => r.height)) };
}
