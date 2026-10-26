import { describe, expect, it } from 'vitest';
import { passedThreshold } from './drag';

describe('passedThreshold', () => {
  it('stays below the threshold under 4 px of total movement', () => {
    expect(passedThreshold(0, 0)).toBe(false);
    expect(passedThreshold(2, -1)).toBe(false);
  });

  it('passes at 4 px of total movement in any direction', () => {
    expect(passedThreshold(4, 0)).toBe(true);
    expect(passedThreshold(-2, 2)).toBe(true);
  });
});
