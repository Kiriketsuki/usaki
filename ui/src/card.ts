// The expanded card: one row per account with both windows. A port of
// docs/design/mockup/Card.dc.html. It holds no DOM code.

import { type HeadMode, type Pin, pickHead } from './centre';
import { DEFAULT_THRESHOLDS, type Thresholds, tone } from './colour';
import { ageText, clockAt, minutesUntil, spacedLeft } from './countdown';
import { type MoonView, NEW_MOON, moonView } from './moon';
import { type AccountView, FIVE_HOUR_MINUTES, SEVEN_DAY_MINUTES, type UsageUpdate, windowAt } from './usage';

/** The card width from the mockup, in px. */
export const CARD_WIDTH = 420;
/** The tri-primary bar on top of the card, in px. */
export const CARD_BAR = 3;
/** The header: 44 px buttons, 4 px padding above and below, and a 1 px rule. */
export const CARD_HEADER = 53;
/**
 * One account block: 12 px padding above and below, a 16 px title line, an 8 px gap, two
 * 16 px window rows with a 6 px gap, and a 1 px rule.
 */
export const CARD_ROW = 87;

/** The colour of the hex dot of a Claude account, from the mockup. */
const CLAUDE_DOT = '#d47622';
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
  dot: string;
  /** `next 5h` on the auto head, `pinned` on a pinned head, else null. */
  tag: string | null;
  /** `stale 3h` for a stale account, else null. The rows then dim. */
  stale: string | null;
  windows: [WindowRow, WindowRow];
}

export interface CardView {
  /** `updated 2m ago`, from the newest data of any account. */
  updated: string;
  rows: AccountRow[];
  width: number;
  height: number;
}

/** Returns the card size for `count` accounts. Zero accounts still get one row, for the no data text. */
export function cardSize(count: number): { width: number; height: number } {
  return { width: CARD_WIDTH, height: CARD_BAR + CARD_HEADER + Math.max(1, count) * CARD_ROW };
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

function headTag(mode: HeadMode): string | null {
  if (mode === 'auto') return 'next 5h';
  if (mode === 'pinned') return 'pinned';
  return null;
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
  const accounts = update?.accounts ?? [];
  const head = pickHead(accounts, pin, now);
  const rows = accounts.map((a) => ({
    id: a.id,
    dot: CLAUDE_DOT,
    tag: a === head.account ? headTag(head.mode) : null,
    stale: staleText(a, now),
    windows: [
      windowRow(a, FIVE_HOUR_MINUTES, now, thresholds, timeZone),
      windowRow(a, SEVEN_DAY_MINUTES, now, thresholds, timeZone)
    ] as [WindowRow, WindowRow]
  }));
  return { updated: updatedText(accounts, now), rows, ...cardSize(accounts.length) };
}
