import { describe, expect, it } from 'vitest';
import { AUTO, keepPin, nextPin, pickHead } from './centre';
import { NOW, account } from './test-fixtures';

// acc2 resets first, so it is the auto head.
const accts = [account('acc1', 42, 133), account('acc2', 81, 47), account('acc3', 5, 90)];

describe('pickHead', () => {
  it('picks the auto account when nothing is pinned', () => {
    expect(pickHead(accts, AUTO, NOW)).toEqual({ account: accts[1], mode: 'auto' });
  });

  it('picks the pinned account, even when it is stale', () => {
    const withStale = [...accts, account('acc4', 1, 10, true)];
    expect(pickHead(withStale, 'acc4', NOW)).toEqual({ account: withStale[3], mode: 'pinned' });
  });

  it('falls back to auto when the pinned account is gone', () => {
    expect(pickHead(accts, 'gone', NOW)).toEqual({ account: accts[1], mode: 'auto' });
  });

  it('puts the first account in the head with no mode when no account is live', () => {
    const stale = [account('acc1', 1, 10, true), account('acc2', 1, 5, true)];
    expect(pickHead(stale, AUTO, NOW)).toEqual({ account: stale[0], mode: 'none' });
    expect(pickHead([], AUTO, NOW)).toEqual({ account: null, mode: 'none' });
  });
});

describe('nextPin', () => {
  it('pins the account after the auto head on the first right-click', () => {
    expect(nextPin(accts, AUTO, NOW)).toBe('acc3');
  });

  it('steps one account per click in account order and wraps, never back to auto', () => {
    const seen: (string | null)[] = [];
    let pin = AUTO;
    for (let i = 0; i < 5; i += 1) {
      pin = nextPin(accts, pin, NOW);
      seen.push(pin);
    }
    expect(seen).toEqual(['acc3', 'acc1', 'acc2', 'acc3', 'acc1']);
  });

  it('steps from the first account when no account is live', () => {
    const stale = [account('acc1', 1, 10, true), account('acc2', 1, 5, true)];
    expect(nextPin(stale, AUTO, NOW)).toBe('acc2');
  });

  it('pins the only account, and stays auto with no account', () => {
    expect(nextPin([account('acc1', 1, 10)], AUTO, NOW)).toBe('acc1');
    expect(nextPin([account('acc1', 1, 10)], 'acc1', NOW)).toBe('acc1');
    expect(nextPin([], AUTO, NOW)).toBe(AUTO);
  });

  it('steps from the auto head when the pinned account is gone', () => {
    expect(nextPin(accts, 'gone', NOW)).toBe('acc3');
  });
});

describe('middle-click and gone accounts', () => {
  it('returns to auto from any pin', () => {
    const pinned = nextPin(accts, AUTO, NOW);
    expect(pickHead(accts, pinned, NOW).mode).toBe('pinned');
    expect(pickHead(accts, AUTO, NOW).mode).toBe('auto');
  });

  it('keeps a pin while its account exists and drops it after', () => {
    expect(keepPin(accts, 'acc1')).toBe('acc1');
    expect(keepPin(accts, AUTO)).toBe(AUTO);
    expect(keepPin(accts.slice(1), 'acc1')).toBe(AUTO);
  });
});
