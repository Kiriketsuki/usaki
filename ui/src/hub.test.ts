import { describe, expect, it } from 'vitest';
import { cardView, forecastText, poolBlock } from './card';
import { bubbleView } from './model';
import { AUTO } from './centre';
import { poolAccount, viewAccounts } from './hub';
import { MIN, NOW, account, codexAccount } from './test-fixtures';
import type { PoolView, SeatView, UsageUpdate } from './usage';

const UTC = 'UTC';
const win = (minutes: number, usedPct: number, resetMins: number) => ({ minutes, usedPct, resetsAt: NOW + resetMins * MIN });

function seat(name: string, over: Partial<SeatView> = {}): SeatView {
  return {
    name,
    priority: 100,
    state: 'open',
    unavailable: false,
    next: false,
    extraUsageOpen: false,
    spendGuardBlocked: false,
    liveSessions: null,
    score: null,
    fiveHour: win(300, 6, 150),
    sevenDay: win(10080, 51, 3000),
    ...over
  };
}

function pool(over: Partial<PoolView> = {}): PoolView {
  return {
    name: 'pool-a',
    source: 'cache',
    updatedAt: NOW,
    fetchedAt: NOW,
    stale: false,
    error: null,
    nextSeat: 'acc1',
    extraUsageOpen: [],
    spendGuardBlocked: [],
    seats: [
      seat('acc1', { next: true, liveSessions: 2 }),
      seat('acc2', { state: 'week_full', unavailable: true, fiveHour: win(300, 90, 60), sevenDay: win(10080, 99, 1000) })
    ],
    total: { fiveHourPct: 48, sevenDayPct: 75, openSeats: 1, seatCount: 2, nextRefillAt: NOW + 1000 * MIN },
    forecast: { runsOutAt: NOW + 28 * 60 * MIN, ratePctPerHour: 0.6, basis: 'recent' },
    ...over
  };
}

const hubUpdate = (over: Partial<UsageUpdate> = {}): UsageUpdate => ({
  polledAt: NOW,
  accounts: [account('acc9', 20, 60), codexAccount('cx1', 30, 3000)],
  error: null,
  mode: 'hub',
  pools: [pool()],
  ...over
});

describe('poolAccount', () => {
  it('builds a synthetic Claude account with the soonest resets', () => {
    const a = poolAccount(pool(), NOW);
    expect(a).toMatchObject({ id: 'hub:pool-a', provider: 'claude', name: 'pool-a', stale: false });
    expect(a.windows).toEqual([
      { minutes: 300, usedPct: 48, resetsAt: NOW + 60 * MIN },
      { minutes: 10080, usedPct: 75, resetsAt: NOW + 1000 * MIN }
    ]);
  });

  it('copies the stale flag and skips a null total', () => {
    const p = pool({ stale: true });
    p.total = { ...p.total, fiveHourPct: null };
    const a = poolAccount(p, NOW);
    expect(a.stale).toBe(true);
    expect(a.windows).toHaveLength(1);
  });
});

describe('viewAccounts', () => {
  it('puts the pool cells after the Claude accounts and before Codex', () => {
    expect(viewAccounts(hubUpdate()).map((a) => a.id)).toEqual(['acc9', 'hub:pool-a', 'codex:cx1']);
  });

  it('returns the accounts unchanged in normal mode or with missing fields', () => {
    const accounts = [account('acc1', 10, 60)];
    expect(viewAccounts({ polledAt: NOW, accounts, error: null })).toBe(accounts);
    expect(viewAccounts({ polledAt: NOW, accounts, error: null, mode: 'normal', pools: [] })).toBe(accounts);
    expect(viewAccounts({ polledAt: NOW, accounts, error: null, mode: 'normal', pools: [pool()] })).toBe(accounts);
  });
});

describe('hub bubble', () => {
  it('draws one extra cell per pool and names the pool in the label', () => {
    const view = bubbleView(hubUpdate({ accounts: [] }), NOW, 'hub:pool-a');
    expect(view.cells.map((c) => c.id)).toEqual(['hub:pool-a']);
    expect(view.label).toContain('Pool pool-a, next seat acc1');
  });

  it('counts the pool cell in the layout', () => {
    expect(bubbleView(hubUpdate(), NOW).cells).toHaveLength(3);
  });
});

describe('forecastText', () => {
  it('says when the pool runs out, with the rate and the basis', () => {
    expect(forecastText(pool(), NOW)).toBe('runs out in 1d 4h, 0.6%/h, last 1h');
  });

  it('says lasts past 7d, and week avg for the week basis', () => {
    const f = { runsOutAt: null, ratePctPerHour: 0.25, basis: 'week' as const };
    expect(forecastText(pool({ forecast: f }), NOW)).toBe('lasts past 7d, 0.3%/h, week avg');
  });

  it('falls back to the pool error when there is no forecast', () => {
    expect(forecastText(pool({ forecast: null, error: 'bad reply' }), NOW)).toBe('bad reply');
  });
});

describe('poolBlock', () => {
  it('lists the title tag, windows, refill and one line per seat', () => {
    const b = poolBlock(pool(), NOW, undefined, UTC);
    expect(b.next).toBe('next acc1');
    expect(b.windows.map((w) => [w.label, w.pct])).toEqual([['5h', '48%'], ['7d', '75%']]);
    expect(b.refill).toBe('refill in 16h 40m');
    expect(b.warnings).toEqual([]);
    expect(b.seats).toHaveLength(2);
    expect(b.seats[0]).toMatchObject({ name: 'acc1', next: true, state: 'open', priority: 'p100', live: '2 live', dim: false });
    expect(b.seats[0].five).toEqual({ pct: '6%', left: '2h 30m' });
  });

  it('dims an unavailable seat and shows no live count when unknown', () => {
    const s = poolBlock(pool(), NOW).seats[1];
    expect(s).toMatchObject({ name: 'acc2', next: false, state: 'week full', dim: true, live: null });
  });

  it('names the seats in the warning lines', () => {
    const b = poolBlock(pool({ extraUsageOpen: ['acc3'], spendGuardBlocked: ['acc2', 'acc4'] }), NOW);
    expect(b.warnings).toEqual(['extra usage: acc3', 'spend guard: acc2, acc4']);
  });
});

describe('hub card', () => {
  it('gives a pool row a pool block and the height of its lines', () => {
    const view = cardView(hubUpdate(), NOW, AUTO, undefined, UTC);
    const row = view.rows.find((r) => r.id === 'hub:pool-a');
    expect(row?.pool).toBeDefined();
    expect(row?.tag).toBe('next acc1');
    // 2 windows + forecast + refill + 2 seats = 6 lines
    expect(row?.height).toBe(49 + 6 * 16 + 5 * 6);
    expect(view.rows.find((r) => r.id === 'acc9')?.pool).toBeUndefined();
  });
});

describe('normal mode', () => {
  it('gives the same bubble and card with or without the hub fields', () => {
    const accounts = [account('acc1', 40, 100), codexAccount('cx1', 30, 3000)];
    const old: UsageUpdate = { polledAt: NOW, accounts, error: null };
    const normal: UsageUpdate = { ...old, mode: 'normal', pools: [] };
    expect(bubbleView(normal, NOW)).toEqual(bubbleView(old, NOW));
    expect(cardView(normal, NOW, AUTO, undefined, UTC)).toEqual(cardView(old, NOW, AUTO, undefined, UTC));
    expect(cardView(old, NOW, AUTO).rows.every((r) => r.pool === undefined)).toBe(true);
  });
});
