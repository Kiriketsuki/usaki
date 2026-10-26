// Synthetic accounts for the tests. No real account data.

import type { AccountView } from './usage';

export const NOW = Date.UTC(2026, 9, 26, 12, 0, 0);
export const MIN = 60_000;

export function account(
  id: string,
  pct: number,
  resetInMins: number | null,
  stale = false,
  sevenPct = 10
): AccountView {
  return {
    id,
    stale,
    fetchedAt: NOW,
    windows: [
      { minutes: 300, usedPct: pct, resetsAt: resetInMins == null ? null : NOW + resetInMins * MIN },
      { minutes: 10080, usedPct: sevenPct, resetsAt: NOW + 3 * 1440 * MIN }
    ],
    error: null
  };
}
