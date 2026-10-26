import { describe, expect, it } from 'vitest';
import { cellText, figures, pickAuto } from './centre';
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
  it('shows the 5h countdown, the name and the 5h and 7d percent', () => {
    expect(cellText(account('acc2', 81, 47, false, 58), NOW)).toEqual({
      countdown: '47m',
      name: 'acc2',
      figures: '81/58%'
    });
  });

  it('rounds both percents', () => {
    expect(cellText(account('acc1', 41.6, 133, false, 57.5), NOW).figures).toBe('42/58%');
  });

  it('shows -- for an unknown reset', () => {
    expect(cellText(account('acc1', 42, null), NOW).countdown).toBe('--');
  });
});

describe('figures', () => {
  it('shows -- for a missing window', () => {
    const a = account('acc1', 42, 133);
    const no7d: AccountView = { ...a, windows: a.windows.filter((w) => w.minutes === 300) };
    expect(figures(no7d, NOW)).toBe('42/--%');
    expect(figures({ ...a, windows: [] }, NOW)).toBe('--/--%');
  });

  it('shows 0 for a 7d window whose reset has passed', () => {
    const a = account('acc1', 42, 133, false, 90);
    const past: AccountView = {
      ...a,
      windows: a.windows.map((w) => (w.minutes === 10080 ? { ...w, resetsAt: NOW - 1 } : w))
    };
    expect(figures(past, NOW)).toBe('42/0%');
  });
});
