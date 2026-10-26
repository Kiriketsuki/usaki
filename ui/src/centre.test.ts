import { describe, expect, it } from 'vitest';
import { NO_DATA, centreText, pickAuto } from './centre';
import { NOW, account } from './test-fixtures';

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

describe('centreText', () => {
  it('shows the mode, the countdown and the short name with the percent', () => {
    const accts = [account('acc1', 42, 133), account('acc2', 81, 47)];
    expect(centreText(accts, NOW)).toEqual({ top: 'next 5h', centre: '47m', sub: 'acc2 · 81%' });
  });

  it('rounds the percent', () => {
    expect(centreText([account('acc1', 41.6, 133)], NOW).sub).toBe('acc1 · 42%');
  });

  it('shows -- for an unknown reset', () => {
    expect(centreText([account('acc1', 42, null)], NOW).centre).toBe('--');
  });

  it('shows no data when no account is live', () => {
    expect(centreText([account('acc1', 42, 10, true)], NOW)).toEqual(NO_DATA);
  });
});
