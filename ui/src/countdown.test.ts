import { describe, expect, it } from 'vitest';
import { ageText, clockAt, left, minutesUntil, spacedLeft } from './countdown';

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

describe('card time text', () => {
  it('spaces the card countdown like the mockup', () => {
    expect(spacedLeft(0)).toBe('now');
    expect(spacedLeft(47)).toBe('47m');
    expect(spacedLeft(133)).toBe('2h 13m');
    expect(spacedLeft(4 * 1440 + 6 * 60 + 20)).toBe('4d 6h');
  });

  it('gives the clock time, with the weekday for a 7d reset', () => {
    const at = Date.UTC(2026, 9, 27, 20, 47);
    expect(clockAt(at, false, 'UTC')).toBe('20:47');
    expect(clockAt(at, true, 'UTC')).toBe('Tue 20:47');
    expect(clockAt(Date.UTC(2026, 9, 27, 0, 5), false, 'UTC')).toBe('00:05');
  });

  it('gives a coarse age, and nothing under a minute', () => {
    expect(ageText(30_000)).toBe('');
    expect(ageText(12 * 60_000)).toBe('12m');
    expect(ageText(47 * 3_600_000)).toBe('47h');
    expect(ageText(50 * 3_600_000)).toBe('2d');
  });
});
