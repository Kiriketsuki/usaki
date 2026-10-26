import { describe, expect, it } from 'vitest';
import { EAR_HEIGHT, EAR_POINTS, EAR_REACH, earState, ears } from './ears';
import { CELL_R } from './geometry';

const parse = (s: string) => s.split(' ').map((p) => p.split(',').map(Number) as [number, number]);

const w = (five: number, seven: number) => ({ five, seven });

describe('earState', () => {
  it('stands both ears when every live window is under crit', () => {
    expect(earState([w(42, 63), w(70, 10)], 75)).toBe('up');
  });

  it('folds the right ear for a live 5h window at crit or more', () => {
    expect(earState([w(42, 10), w(75, 20)], 75)).toBe('right');
  });

  it('folds the left ear for a live 7d window at crit or more', () => {
    expect(earState([w(42, 10), w(20, 75)], 75)).toBe('left');
  });

  it('folds both ears when a 5h and a 7d window are at crit, in one or two accounts', () => {
    expect(earState([w(80, 90)], 75)).toBe('both');
    expect(earState([w(80, 10), w(10, 90)], 75)).toBe('both');
  });

  it('folds both ears for a full window of either length', () => {
    expect(earState([w(42, 10), w(100, 10)], 75)).toBe('both');
    expect(earState([w(42, 10), w(10, 100)], 75)).toBe('both');
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

  it('folds the right ear only in the right state, with a darker hanging flap', () => {
    const [l, r] = ears('right', 0, 0);
    expect(l.facets).toHaveLength(3);
    expect(r.facets).toHaveLength(5);
    expect(r.facets[4].fill).toBe('#1c1f2b');
  });

  it('folds the left ear only in the left state, as the mirror of a right fold', () => {
    const [l, r] = ears('left', 0, 0);
    expect(l.facets).toHaveLength(5);
    expect(r.facets).toHaveLength(3);
    const [, rFold] = ears('right', 0, 0);
    expect(parse(l.edge)).toEqual(parse(rFold.edge).map(([x, y]) => [-x + 0, y]));
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
