import { describe, expect, it } from 'vitest';
import { MOON_DISC, moonLit, moonPhase, moonView } from './moon';

describe('moon', () => {
  it('is new just after a reset and full at the reset', () => {
    expect(moonPhase(10080)).toBe(0);
    expect(moonPhase(5040)).toBe(0.5);
    expect(moonPhase(0)).toBe(1);
    expect(moonPhase(-30)).toBe(1);
    expect(moonPhase(20000)).toBe(0);
  });

  it('draws no lit part for a new moon', () => {
    expect(moonLit(0)).toBe('');
  });

  it('draws a crescent under half and a gibbous moon over half', () => {
    expect(moonLit(0.25)).toBe('M 6 1 A 5 5 0 0 1 6 11 A 2.50 5 0 0 0 6 1 Z');
    expect(moonLit(0.75)).toBe('M 6 1 A 5 5 0 0 1 6 11 A 2.50 5 0 0 1 6 1 Z');
    expect(moonLit(1)).toBe('M 6 1 A 5 5 0 0 1 6 11 A 5.00 5 0 0 1 6 1 Z');
  });

  it('draws the disc as two half arcs and labels the phase', () => {
    expect(MOON_DISC).toBe('M 6 1 A 5 5 0 0 1 6 11 A 5 5 0 0 1 6 1 Z');
    expect(moonView(2520, '1d 18h').label).toBe('Moon 75% full, 7d reset in 1d 18h');
  });
});
