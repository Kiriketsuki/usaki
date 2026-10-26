// The expanded card: one row per account with both windows. A port of
// docs/design/mockup/Card.dc.html. It holds no DOM code.

import { type Head, type Pin, pickHead } from './centre';
import { DEFAULT_THRESHOLDS, type Thresholds, tone } from './colour';
import { ageText, clockAt, minutesUntil, spacedLeft } from './countdown';
import { PROVIDER_COLOUR } from './marks';
import { type MoonView, NEW_MOON, moonView } from './moon';
import {
  type AccountView,
  type Credits,
  FIVE_HOUR_MINUTES,
  SEVEN_DAY_MINUTES,
  type UsageUpdate,
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
  /** The colour of the hex dot: amethyst for Claude, teal for Codex. */
  dot: string;
  /** `next 5h` or `next 7d` on the auto head, `pinned` on a pinned head, else null. */
  tag: string | null;
  /** `stale 3h` for a stale account, else null. The rows then dim. */
  stale: string | null;
  /** The 5h row and the 7d row. A 7d-only account has the 7d row only. */
  windows: WindowRow[];
  /** The credits balance as the provider gives it, `unlimited`, or null for no credits row. */
  credits: string | null;
  /** The block height in px. */
  height: number;
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
  if (head.mode === 'auto' && head.account) return weekOnly(head.account) ? 'next 7d' : 'next 5h';
  if (head.mode === 'pinned') return 'pinned';
  return null;
}

/** Returns the text of the credits row, or null when the account reports no credits. */
export function creditsText(credits: Credits | null): string | null {
  if (!credits) return null;
  if (credits.unlimited) return 'unlimited';
  return credits.balance;
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
  const rows = accounts.map((a): AccountRow => {
    const minutes = weekOnly(a) ? [SEVEN_DAY_MINUTES] : [FIVE_HOUR_MINUTES, SEVEN_DAY_MINUTES];
    const windows = minutes.map((m) => windowRow(a, m, now, thresholds, timeZone));
    const credits = creditsText(a.credits);
    return {
      id: a.id,
      name: a.name,
      dot: PROVIDER_COLOUR[a.provider],
      tag: a === head.account ? headTag(head) : null,
      stale: staleText(a, now),
      windows,
      credits,
      height: blockHeight(windows.length + (credits == null ? 0 : 1))
    };
  });
  return { updated: updatedText(accounts, now), rows, ...cardSize(rows.map((r) => r.height)) };
}
