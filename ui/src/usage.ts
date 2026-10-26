// The usage data from the backend. It matches src-tauri/src/view.rs. Times are epoch ms.

/** The length of the 5h window in minutes. */
export const FIVE_HOUR_MINUTES = 300;
/** The length of the 7d window in minutes. */
export const SEVEN_DAY_MINUTES = 10080;

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

/** One window of an account as it stands at `now`. */
export interface WindowNow {
  pct: number;
  resetsAt: number | null;
}

/**
 * Returns the window of `minutes` length at `now`, or null when the account has no such
 * window. A window whose reset time has passed has reset, so it reports 0%. This matches
 * Window::as_of in usaki-core, and it keeps the ring correct between two polls.
 */
export function windowAt(account: AccountView, minutes: number, now: number): WindowNow | null {
  const w = account.windows.find((x) => x.minutes === minutes);
  if (!w) return null;
  const reset = w.resetsAt != null && w.resetsAt <= now;
  return { pct: reset ? 0 : w.usedPct, resetsAt: w.resetsAt };
}

const NO_WINDOW: WindowNow = { pct: 0, resetsAt: null };

/** Returns the 5h window of an account at `now`. A missing window reports 0%. */
export function fiveHour(account: AccountView, now: number): WindowNow {
  return windowAt(account, FIVE_HOUR_MINUTES, now) ?? NO_WINDOW;
}

/** Returns the 7d window of an account at `now`. A missing window reports 0%. */
export function sevenDay(account: AccountView, now: number): WindowNow {
  return windowAt(account, SEVEN_DAY_MINUTES, now) ?? NO_WINDOW;
}
