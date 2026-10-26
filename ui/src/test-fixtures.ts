// Synthetic accounts for the tests. No real account data.

import type { AccountView } from './usage';

export const NOW = Date.UTC(2026, 9, 26, 12, 0, 0);
export const MIN = 60_000;

export function account(
  id: string,
  pct: number,
  resetInMins: number | null,
  stale = false,
  sevenPct = 10,
  sevenResetMins: number | null = 3 * 1440
): AccountView {
  return {
    id,
    provider: 'claude',
    name: id,
    stale,
    fetchedAt: NOW,
    windows: [
      { minutes: 300, usedPct: pct, resetsAt: resetInMins == null ? null : NOW + resetInMins * MIN },
      { minutes: 10080, usedPct: sevenPct, resetsAt: sevenResetMins == null ? null : NOW + sevenResetMins * MIN }
    ],
    error: null,
    credits: null
  };
}

/** A Codex account with a 7d window only, as the Pro plan reports it. */
export function codexAccount(
  name: string,
  sevenPct: number,
  sevenResetMins: number | null,
  stale = false,
  balance: string | null = '123.45'
): AccountView {
  return {
    id: 'codex:' + name,
    provider: 'codex',
    name,
    stale,
    fetchedAt: NOW,
    windows: [
      { minutes: 10080, usedPct: sevenPct, resetsAt: sevenResetMins == null ? null : NOW + sevenResetMins * MIN }
    ],
    error: null,
    credits: balance == null ? null : { hasCredits: true, unlimited: false, balance }
  };
}
