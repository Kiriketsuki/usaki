import { describe, expect, it } from 'vitest';
import { left, minutesUntil } from './countdown';

describe('left', () => {
  it('says now at or after the reset', () => {
    expect(left(0)).toBe('now');
    expect(left(-3)).toBe('now');
  });

  it('shows minutes under an hour and rounds up', () => {
    expect(left(33)).toBe('33m');
    expect(left(0.2)).toBe('1m');
    expect(left(59.5)).toBe('1h00m');
  });

  it('shows hours and padded minutes under a day, with no space', () => {
    expect(left(72)).toBe('1h12m');
    expect(left(65)).toBe('1h05m');
    expect(left(60)).toBe('1h00m');
    expect(left(1439)).toBe('23h59m');
  });

  it('shows days and padded hours from a day, with no space', () => {
    expect(left(2 * 1440 + 4 * 60 + 20)).toBe('2d04h');
    expect(left(1440)).toBe('1d00h');
    expect(left(6 * 1440 + 23 * 60 + 59)).toBe('6d23h');
  });
});

describe('minutesUntil', () => {
  it('converts epoch ms to minutes', () => {
    expect(minutesUntil(1_000 + 90 * 60_000, 1_000)).toBe(90);
  });
});
