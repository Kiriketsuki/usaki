import { describe, expect, it } from 'vitest';
import { BLONDE, EMERALD, RUBY, tone } from './colour';

describe('tone', () => {
  it('is emerald under 50%', () => {
    expect(tone(0)).toBe(EMERALD);
    expect(tone(49.9)).toBe(EMERALD);
  });

  it('is blonde from 50%', () => {
    expect(tone(50)).toBe(BLONDE);
    expect(tone(74.9)).toBe(BLONDE);
  });

  it('is ruby from 75%', () => {
    expect(tone(75)).toBe(RUBY);
    expect(tone(120)).toBe(RUBY);
  });

  it('uses the thresholds it gets', () => {
    expect(tone(30, { warn: 20, crit: 40 })).toBe(BLONDE);
    expect(tone(40, { warn: 20, crit: 40 })).toBe(RUBY);
  });
});
