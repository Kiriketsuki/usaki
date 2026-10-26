import { describe, expect, it } from 'vitest';
import { left, minutesUntil } from './countdown';

describe('left', () => {
  it('says now at or after the reset', () => {
    expect(left(0)).toBe('now');
    expect(left(-3)).toBe('now');
  });

  it('shows minutes under an hour and rounds up', () => {
    expect(left(47)).toBe('47m');
    expect(left(0.2)).toBe('1m');
  });

  it('shows hours and padded minutes under a day', () => {
    expect(left(133)).toBe('2h 13m');
    expect(left(65)).toBe('1h 05m');
    expect(left(60)).toBe('1h 00m');
  });

  it('shows days and hours from a day', () => {
    expect(left(4 * 1440 + 6 * 60 + 20)).toBe('4d 6h');
    expect(left(1440)).toBe('1d 0h');
  });
});

describe('minutesUntil', () => {
  it('converts epoch ms to minutes', () => {
    expect(minutesUntil(1_000 + 90 * 60_000, 1_000)).toBe(90);
  });
});
