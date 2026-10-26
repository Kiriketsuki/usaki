// Centre selection and text, ported from docs/design/mockup/Main.dc.html. Milestone 1
// has the auto mode only. Right-click and middle-click come in milestone 2.

import { left, minutesUntil } from './countdown';
import { type AccountView, FIVE_HOUR_MINUTES, SEVEN_DAY_MINUTES, fiveHour, windowAt } from './usage';

/**
 * The three centre lines. The account line is `name` and `figures` with a space between.
 * The name fit may cut the name, but it never cuts the figures.
 */
export interface CentreText {
  top: string;
  centre: string;
  name: string;
  figures: string;
}

/** The text of the States artboard "Stale" cell: no live account to show. */
export const NO_DATA: CentreText = { top: 'no data', centre: '--', name: 'offline', figures: '' };

/** Joins a name and the figures into the account line. */
export function accountLine(name: string, figures: string): string {
  return figures ? name + ' ' + figures : name;
}

function figure(account: AccountView, minutes: number, now: number): string {
  const w = windowAt(account, minutes, now);
  return w ? String(Math.round(w.pct)) : '--';
}

/** The 5h and the 7d percent as `81/58%`. A missing window shows `--`. */
export function figures(account: AccountView, now: number): string {
  return figure(account, FIVE_HOUR_MINUTES, now) + '/' + figure(account, SEVEN_DAY_MINUTES, now) + '%';
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

/** The three centre lines: the mode, the countdown, and `<name> <5h>/<7d>%`. */
export function centreText(accounts: readonly AccountView[], now: number): CentreText {
  const shown = pickAuto(accounts, now);
  if (!shown) return NO_DATA;
  const five = fiveHour(shown, now);
  return {
    top: 'next 5h',
    centre: five.resetsAt == null ? '--' : left(minutesUntil(five.resetsAt, now)),
    name: shown.id,
    figures: figures(shown, now)
  };
}
