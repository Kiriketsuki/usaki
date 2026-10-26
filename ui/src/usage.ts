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

/** The provider of an account. It is the first part of the wire id. */
export type ProviderName = 'claude' | 'codex';

/** The credits state of an account. The balance is the string that the provider gives. */
export interface Credits {
  hasCredits: boolean;
  unlimited: boolean;
  balance: string | null;
}

export interface AccountView {
  /** The wire id, `<provider>:<name>`. The page keys pins by it. */
  id: string;
  provider: ProviderName;
  /** The account name that the cell shows. */
  name: string;
  stale: boolean;
  fetchedAt: number;
  windows: WindowView[];
  error: string | null;
  credits: Credits | null;
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
  /** True when no countdown runs: the window has no reset time, or its reset has passed. */
  ready: boolean;
}

/**
 * Returns the window of `minutes` length at `now`, or null when the account has no such
 * window. A window whose reset time has passed has reset, so it reports 0%. This matches
 * Window::as_of in usaki-core, and it keeps the ring correct between two polls. Claude sends
 * a null resets_at for a 5h window that has not started yet. Such a window also reports 0%.
 */
export function windowAt(account: AccountView, minutes: number, now: number): WindowNow | null {
  const w = account.windows.find((x) => x.minutes === minutes);
  if (!w) return null;
  const ready = w.resetsAt == null || w.resetsAt <= now;
  return { pct: ready ? 0 : w.usedPct, resetsAt: w.resetsAt, ready };
}

const NO_WINDOW: WindowNow = { pct: 0, resetsAt: null, ready: true };

/** Returns the 5h window of an account at `now`. A missing window reports 0%. */
export function fiveHour(account: AccountView, now: number): WindowNow {
  return windowAt(account, FIVE_HOUR_MINUTES, now) ?? NO_WINDOW;
}

/** Returns the 7d window of an account at `now`. A missing window reports 0%. */
export function sevenDay(account: AccountView, now: number): WindowNow {
  return windowAt(account, SEVEN_DAY_MINUTES, now) ?? NO_WINDOW;
}

/**
 * True for an account with a 7d window and no 5h window, such as Codex on the Pro plan. The
 * cell then has no 5h ring, the 5h slots show `--`, and the head rule ranks the account last.
 */
export function weekOnly(account: AccountView): boolean {
  const has = (minutes: number) => account.windows.some((w) => w.minutes === minutes);
  return has(SEVEN_DAY_MINUTES) && !has(FIVE_HOUR_MINUTES);
}
