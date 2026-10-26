// The usage data from the backend. It matches src-tauri/src/view.rs. Times are epoch ms.

/** The length of the 5h window in minutes. */
export const FIVE_HOUR_MINUTES = 300;

export interface WindowView {
  minutes: number;
  usedPct: number;
  resetsAt: number | null;
}

export interface AccountView {
  id: string;
  stale: boolean;
  fetchedAt: number;
  windows: WindowView[];
  error: string | null;
}

export interface UsageUpdate {
  polledAt: number;
  accounts: AccountView[];
  error: string | null;
}

/** The 5h window of an account as it stands at `now`. */
export interface FiveHour {
  pct: number;
  resetsAt: number | null;
}

/**
 * Returns the 5h window of an account at `now`. A window whose reset time has passed has
 * reset, so it reports 0%. This matches Window::as_of in usaki-core, and it keeps the
 * ring correct between two polls. An account without a 5h window reports 0%.
 */
export function fiveHour(account: AccountView, now: number): FiveHour {
  const w = account.windows.find((x) => x.minutes === FIVE_HOUR_MINUTES);
  if (!w) return { pct: 0, resetsAt: null };
  const reset = w.resetsAt != null && w.resetsAt <= now;
  return { pct: reset ? 0 : w.usedPct, resetsAt: w.resetsAt };
}
