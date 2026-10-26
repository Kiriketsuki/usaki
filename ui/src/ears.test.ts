import { describe, expect, it } from 'vitest';
import { earState, ears } from './ears';

describe('earState', () => {
  it('stands both ears when every live ring is under crit', () => {
    expect(earState([42, 63], 75)).toBe('up');
  });

  it('folds one ear when a live ring is at crit or more', () => {
    expect(earState([42, 75], 75)).toBe('one');
  });

  it('folds both ears for a full ring', () => {
    expect(earState([42, 100], 75)).toBe('both');
  });

  it('folds both ears when no account is live', () => {
    expect(earState([], 75)).toBe('both');
  });
});

describe('ears', () => {
  it('mirrors the right ear around the 152 px width', () => {
    const [l, r] = ears('up');
    expect(l.edge).toBe('45,48 36,20 44,2 60,14 67,48');
    expect(r.edge).toBe('107,48 116,20 108,2 92,14 85,48');
  });

  it('folds the right ear only in the one state', () => {
    const [l, r] = ears('one');
    expect(l.facets).toHaveLength(3);
    expect(r.facets).toHaveLength(5);
  });

  it('folds both ears in the both state', () => {
    expect(ears('both').map((e) => e.facets.length)).toEqual([5, 5]);
  });
});
