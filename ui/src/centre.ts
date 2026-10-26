// The head pick and the cell text. The auto pick and the cycle come from
// docs/design/mockup/Main.dc.html. A right-click pins the next account in the head, and a
// middle-click returns the head to auto.

import { left, minutesUntil } from './countdown';
import {
  type AccountView,
  FIVE_HOUR_MINUTES,
  type ProviderName,
  SEVEN_DAY_MINUTES,
  type WindowNow,
  fiveHour,
  noData,
  sevenDay,
  weekOnly,
  windowAt
} from './usage';

/** The percent and the countdown of one window, as the cell shows them. */
export interface WindowText {
  pct: string;
  countdown: string;
}

/**
 * The text of one cell: the provider mark and the account name, then the 5h window on the
 * left and the 7d window on the right.
 */
export interface CellText {
  /** The provider of the name mark, or null for a cell with no account. */
  provider: ProviderName | null;
  name: string;
  /** True for an account with no 5h and no 7d window. The cell then shows `no data` under the name. */
  noData: boolean;
  five: WindowText;
  seven: WindowText;
}

/** The countdown of a window with no reset time or a reset that has passed. */
export const READY = 'ready';

const MISSING: WindowText = { pct: '--', countdown: '--' };

/** The text of the one cell that shows when no account exists. */
export const NO_DATA: CellText = { provider: null, name: 'no data', noData: false, five: MISSING, seven: MISSING };

/** The text under the name of an account with no window. */
export const NO_DATA_TEXT = 'no data';

/** The text of one window. A missing window shows `--`. A ready window shows 0% and `ready`. */
export function windowText(w: WindowNow | null, now: number): WindowText {
  if (!w) return MISSING;
  return {
    pct: Math.round(w.pct) + '%',
    countdown: w.ready || w.resetsAt == null ? READY : left(minutesUntil(w.resetsAt, now))
  };
}

/** The text of the cell of `account` at `now`. */
export function cellText(account: AccountView, now: number): CellText {
  return {
    provider: account.provider,
    name: account.name,
    noData: noData(account),
    five: windowText(windowAt(account, FIVE_HOUR_MINUTES, now), now),
    seven: windowText(windowAt(account, SEVEN_DAY_MINUTES, now), now)
  };
}

/** The rank of an account in the auto pick: a tier, then a reset time. Lower comes first. */
type Rank = readonly [number, number];

// An account with a 5h window ranks by its 5h reset. A 7d-only account ranks after every
// account with a 5h window, by its 7d reset. An account with no window ranks last. A reset
// that has passed or that is not known cannot be the next reset, so it sorts last in its tier.
function rank(account: AccountView, now: number): Rank {
  if (noData(account)) return [2, Number.POSITIVE_INFINITY];
  const week = weekOnly(account);
  const at = (week ? sevenDay : fiveHour)(account, now).resetsAt;
  return [week ? 1 : 0, at != null && at > now ? at : Number.POSITIVE_INFINITY];
}

function before(a: Rank, b: Rank): boolean {
  return a[0] < b[0] || (a[0] === b[0] && a[1] < b[1]);
}

/**
 * Auto mode: the live account with the earliest 5h reset, or null when no account is live.
 * A 7d-only account becomes the auto account only when no live account has a 5h window.
 */
export function pickAuto(accounts: readonly AccountView[], now: number): AccountView | null {
  return accounts
    .filter((a) => !a.stale)
    .reduce<AccountView | null>((best, a) => (best == null || before(rank(a, now), rank(best, now)) ? a : best), null);
}

/**
 * The head mode. Null is auto: the head is the live account with the earliest 5h reset. An
 * account id pins that account in the head.
 */
export type Pin = string | null;

/**
 * How the head was chosen. `auto` and `pinned` get a highlight. `none` means auto mode with
 * no live account, so the first account fills the head with no highlight.
 */
export type HeadMode = 'auto' | 'pinned' | 'none';

export interface Head {
  account: AccountView | null;
  mode: HeadMode;
}

/** Returns the head at `now`. A pin whose account is gone falls back to auto. */
export function pickHead(accounts: readonly AccountView[], pin: Pin, now: number): Head {
  const pinned = pin == null ? undefined : accounts.find((a) => a.id === pin);
  if (pinned) return { account: pinned, mode: 'pinned' };
  const auto = pickAuto(accounts, now);
  if (auto) return { account: auto, mode: 'auto' };
  return { account: accounts[0] ?? null, mode: 'none' };
}

/**
 * Right-click: pins the account after the current head, in account order. It wraps from the
 * last account to the first and never returns to auto. With no account it stays auto.
 */
export function nextPin(accounts: readonly AccountView[], pin: Pin, now: number): Pin {
  if (accounts.length === 0) return null;
  const head = pickHead(accounts, pin, now).account;
  const from = head ? accounts.indexOf(head) : -1;
  return accounts[(from + 1) % accounts.length].id;
}

/** Middle-click: returns the head to auto. */
export const AUTO: Pin = null;

/** Keeps a pin only while its account exists. A pin on a gone account becomes auto. */
export function keepPin(accounts: readonly AccountView[], pin: Pin): Pin {
  return pin != null && accounts.some((a) => a.id === pin) ? pin : AUTO;
}
