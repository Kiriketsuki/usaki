import { describe, expect, it } from 'vitest';
import { EAR_HEIGHT, EAR_POINTS, EAR_REACH, earState, ears } from './ears';
import { CELL_R } from './geometry';

const parse = (s: string) => s.split(' ').map((p) => p.split(',').map(Number) as [number, number]);

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
  it('mirrors the right ear around the middle of the top cell', () => {
    const [l, r] = ears('up', 100, 60);
    const lp = parse(l.edge);
    const rp = parse(r.edge);
    lp.forEach(([x, y], i) => expect(rp[i]).toEqual([200 - x, y]));
  });

  it('stands each ear base on the top edge of the cell', () => {
    const [l] = ears('up', 100, 60);
    const base = parse(l.edge).filter(([, y]) => y === 60);
    expect(base).toHaveLength(2);
    base.forEach(([x]) => expect(Math.abs(x - 100)).toBeLessThanOrEqual(CELL_R / 2));
  });

  it('makes the standing ear EAR_HEIGHT tall and tilted outward', () => {
    const [l] = ears('up', 0, 0);
    const pts = parse(l.edge);
    const tip = pts.reduce((a, b) => (b[1] < a[1] ? b : a));
    expect(tip[1]).toBe(-EAR_HEIGHT);
    const baseMid = (pts[0][0] + pts[pts.length - 1][0]) / 2;
    expect(tip[0]).toBeLessThan(baseMid);
  });

  it('folds the right ear only in the one state, with a darker hanging flap', () => {
    const [l, r] = ears('one', 0, 0);
    expect(l.facets).toHaveLength(3);
    expect(r.facets).toHaveLength(5);
    expect(r.facets[4].fill).toBe('#1c1f2b');
  });

  it('folds both ears in the both state, lower than a standing ear', () => {
    const both = ears('both', 0, 0);
    expect(both.map((e) => e.facets.length)).toEqual([5, 5]);
    const top = Math.min(...parse(both[0].edge).map(([, y]) => y));
    expect(top).toBeGreaterThan(-EAR_HEIGHT);
  });

  it('keeps every point within EAR_REACH and EAR_HEIGHT', () => {
    for (const [x, y] of EAR_POINTS) {
      expect(Math.abs(x)).toBeLessThanOrEqual(EAR_REACH);
      expect(y).toBeGreaterThanOrEqual(-EAR_HEIGHT);
      expect(y).toBeLessThanOrEqual(0);
    }
  });
});
