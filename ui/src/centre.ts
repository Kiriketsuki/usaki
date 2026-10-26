// Centre selection and text, ported from docs/design/mockup/Main.dc.html. Milestone 1
// has the auto mode only. Right-click and middle-click come in milestone 2.

import { left, minutesUntil } from './countdown';
import { type AccountView, fiveHour } from './usage';

export interface CentreText {
  top: string;
  centre: string;
  sub: string;
}

/** The text of the States artboard "Stale" cell: no live account to show. */
export const NO_DATA: CentreText = { top: 'no data', centre: '--', sub: 'offline' };

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

/** The three centre lines: the mode, the countdown, and `<short> · <pct>%`. */
export function centreText(accounts: readonly AccountView[], now: number): CentreText {
  const shown = pickAuto(accounts, now);
  if (!shown) return NO_DATA;
  const five = fiveHour(shown, now);
  return {
    top: 'next 5h',
    centre: five.resetsAt == null ? '--' : left(minutesUntil(five.resetsAt, now)),
    sub: shown.id + ' · ' + Math.round(five.pct) + '%'
  };
}
