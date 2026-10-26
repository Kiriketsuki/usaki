import { describe, expect, it } from 'vitest';
import { READY, cellText, pickAuto, windowText } from './centre';
import { NOW, account } from './test-fixtures';
import type { AccountView } from './usage';

describe('pickAuto', () => {
  it('picks the live account with the earliest 5h reset', () => {
    const accts = [account('acc1', 42, 133), account('acc2', 81, 47)];
    expect(pickAuto(accts, NOW)?.id).toBe('acc2');
  });

  it('skips stale accounts', () => {
    const accts = [account('acc1', 42, 133), account('acc2', 81, 47, true)];
    expect(pickAuto(accts, NOW)?.id).toBe('acc1');
  });

  it('puts an unknown or past reset after a known one', () => {
    expect(pickAuto([account('acc1', 42, null), account('acc2', 10, 200)], NOW)?.id).toBe('acc2');
    expect(pickAuto([account('acc1', 42, -5), account('acc2', 10, 200)], NOW)?.id).toBe('acc2');
  });

  it('returns null when no account is live', () => {
    expect(pickAuto([account('acc1', 42, 10, true)], NOW)).toBeNull();
    expect(pickAuto([], NOW)).toBeNull();
  });
});

describe('cellText', () => {
  it('shows the name, then the 5h and the 7d percent and countdown', () => {
    expect(cellText(account('acc2', 33, 72, false, 58, 2 * 1440 + 4 * 60), NOW)).toEqual({
      provider: 'claude',
      name: 'acc2',
      noData: false,
      five: { pct: '33%', countdown: '1h12m' },
      seven: { pct: '58%', countdown: '2d04h' }
    });
  });

  it('rounds both percents', () => {
    const t = cellText(account('acc1', 41.6, 133, false, 57.5), NOW);
    expect([t.five.pct, t.seven.pct]).toEqual(['42%', '58%']);
  });

  it('shows ready and 0% for a 5h window with no reset time', () => {
    expect(cellText(account('acc1', 0, null, false, 100), NOW).five).toEqual({ pct: '0%', countdown: READY });
  });

  it('shows ready and 0% for a window whose reset has passed', () => {
    expect(cellText(account('acc1', 90, -1), NOW).five).toEqual({ pct: '0%', countdown: READY });
  });

  it('shows a full 7d window as 100% with its countdown', () => {
    expect(cellText(account('acc1', 0, null, false, 100, 30), NOW).seven).toEqual({ pct: '100%', countdown: '30m' });
  });
});

describe('windowText', () => {
  it('shows -- for a missing window', () => {
    expect(windowText(null, NOW)).toEqual({ pct: '--', countdown: '--' });
  });

  it('shows the countdown under a day in hours and minutes', () => {
    const a: AccountView = account('acc1', 20, 10, false, 50, 23 * 60 + 59);
    expect(cellText(a, NOW).seven.countdown).toBe('23h59m');
  });
});
