// Hub mode: each Claude hub pool becomes one synthetic account, so a pool cell takes part in
// the layout, the head pick and the ears like any account. This module holds no DOM code.

import { type AccountView, FIVE_HOUR_MINUTES, SEVEN_DAY_MINUTES, type PoolView, type UsageUpdate, type WindowView } from './usage';

/** The id prefix of a pool cell. */
export const HUB_PREFIX = 'hub:';

/** Returns the soonest reset of the windows, or null. A reset in the future wins over a past one. */
function soonest(windows: readonly (WindowView | null)[], now: number): number | null {
  const times = windows.flatMap((w) => (w?.resetsAt == null ? [] : [w.resetsAt]));
  const future = times.filter((t) => t > now);
  const pool = future.length ? future : times;
  return pool.length ? Math.min(...pool) : null;
}

/** Returns the synthetic account of a pool. `now` picks the soonest reset that has not passed. */
export function poolAccount(pool: PoolView, now: number): AccountView {
  const windows: WindowView[] = [];
  if (pool.total.fiveHourPct != null) {
    windows.push({
      minutes: FIVE_HOUR_MINUTES,
      usedPct: pool.total.fiveHourPct,
      resetsAt: soonest(pool.seats.map((s) => s.fiveHour), now)
    });
  }
  if (pool.total.sevenDayPct != null) {
    windows.push({
      minutes: SEVEN_DAY_MINUTES,
      usedPct: pool.total.sevenDayPct,
      resetsAt: soonest(pool.seats.map((s) => s.sevenDay), now)
    });
  }
  return {
    id: HUB_PREFIX + pool.name,
    provider: 'claude',
    name: pool.name,
    stale: pool.stale,
    fetchedAt: pool.fetchedAt,
    windows,
    error: pool.error,
    credits: null
  };
}

/** Returns the pools of an update. A missing `pools` reads as none. */
export function poolsOf(update: UsageUpdate | null): readonly PoolView[] {
  return update?.pools ?? [];
}

/** True when the update is in hub mode and has at least one pool. */
export function isHub(update: UsageUpdate | null): boolean {
  return update?.mode === 'hub' && poolsOf(update).length > 0;
}

/**
 * Returns the accounts that the page draws. In normal mode this is `update.accounts` itself.
 * In hub mode the pool cells follow the Claude accounts and come before the other providers.
 * The poll time of the update picks the soonest reset of each pool window.
 */
export function viewAccounts(update: UsageUpdate | null): AccountView[] {
  const accounts = update?.accounts ?? [];
  if (!update || !isHub(update)) return accounts;
  const cells = poolsOf(update).map((p) => poolAccount(p, update.polledAt));
  const claude = accounts.filter((a) => a.provider === 'claude');
  const rest = accounts.filter((a) => a.provider !== 'claude');
  return [...claude, ...cells, ...rest];
}

/** Returns the pool of a pool cell id, or null for an account id. */
export function poolForId(update: UsageUpdate | null, id: string): PoolView | null {
  if (!id.startsWith(HUB_PREFIX)) return null;
  return poolsOf(update).find((p) => HUB_PREFIX + p.name === id) ?? null;
}
