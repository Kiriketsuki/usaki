// The auto pick and the cell text. The auto pick comes from docs/design/mockup/Main.dc.html.
// Milestone 1 has the auto mode only. Right-click and middle-click come in milestone 2.

import { left, minutesUntil } from './countdown';
import { type AccountView, FIVE_HOUR_MINUTES, SEVEN_DAY_MINUTES, fiveHour, windowAt } from './usage';

/** The three lines of one cell: the 5h countdown, the account name, and the figures. */
export interface CellText {
  countdown: string;
  name: string;
  figures: string;
}

/** The text of the one cell that shows when no account exists. */
export const NO_DATA: CellText = { countdown: '--', name: 'no data', figures: 'offline' };

function figure(account: AccountView, minutes: number, now: number): string {
  const w = windowAt(account, minutes, now);
  return w ? String(Math.round(w.pct)) : '--';
}

/** The 5h and the 7d percent as `81/58%`. A missing window shows `--`. */
export function figures(account: AccountView, now: number): string {
  return figure(account, FIVE_HOUR_MINUTES, now) + '/' + figure(account, SEVEN_DAY_MINUTES, now) + '%';
}

/** The text of the cell of `account` at `now`. */
export function cellText(account: AccountView, now: number): CellText {
  const five = fiveHour(account, now);
  return {
    countdown: five.resetsAt == null ? '--' : left(minutesUntil(five.resetsAt, now)),
    name: account.id,
    figures: figures(account, now)
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
