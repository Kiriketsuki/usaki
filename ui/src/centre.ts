// The auto pick and the cell text. The auto pick comes from docs/design/mockup/Main.dc.html.
// Milestone 1 has the auto mode only. Right-click and middle-click come in milestone 2.

import { left, minutesUntil } from './countdown';
import { type AccountView, FIVE_HOUR_MINUTES, SEVEN_DAY_MINUTES, type WindowNow, fiveHour, windowAt } from './usage';

/** The percent and the countdown of one window, as the cell shows them. */
export interface WindowText {
  pct: string;
  countdown: string;
}

/** The text of one cell: the account name, then the 5h window on the left and the 7d window on the right. */
export interface CellText {
  name: string;
  five: WindowText;
  seven: WindowText;
}

/** The countdown of a window with no reset time or a reset that has passed. */
export const READY = 'ready';

const MISSING: WindowText = { pct: '--', countdown: '--' };

/** The text of the one cell that shows when no account exists. */
export const NO_DATA: CellText = { name: 'no data', five: MISSING, seven: MISSING };

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
    name: account.id,
    five: windowText(windowAt(account, FIVE_HOUR_MINUTES, now), now),
    seven: windowText(windowAt(account, SEVEN_DAY_MINUTES, now), now)
  };
}

// A reset that has passed or that is not known cannot be the next reset, so it sorts last.
function nextResetKey(account: AccountView, now: number): number {
  const at = fiveHour(account, now).resetsAt;
  return at != null && at > now ? at : Number.POSITIVE_INFINITY;
}

/** Auto mode: the live account with the earliest 5h reset, or null when no account is live. */
export function pickAuto(accounts: readonly AccountView[], now: number): AccountView | null {
  return accounts
    .filter((a) => !a.stale)
    .reduce<AccountView | null>(
      (best, a) => (best == null || nextResetKey(a, now) < nextResetKey(best, now) ? a : best),
      null
    );
}
