import { describe, expect, it } from 'vitest';
import {
  EAR_DOT_GAP,
  EAR_DOT_R,
  EAR_LIFT,
  type EarCell,
  allEarCells,
  earCells,
  earFacets,
  earState,
  ears,
  insidePolygon
} from './ears';
import { APOTHEM, HEAD_R, cellGap, hexCorners } from './geometry';

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

describe('insidePolygon', () => {
  it('holds a point inside a square and not one outside it', () => {
    const square: [number, number][] = [[0, 0], [10, 0], [10, 10], [0, 10]];
    expect(insidePolygon(5, 5, square)).toBe(true);
    expect(insidePolygon(15, 5, square)).toBe(false);
  });
});

describe('earCells', () => {
  const CX = 100;
  const TOP = 200;
  const head = { x: CX, y: TOP + HEAD_R * APOTHEM, r: HEAD_R };
  const CREASE = '#1c1f2b';

  for (const folded of [false, true]) {
    describe(folded ? 'a folded ear' : 'a standing ear', () => {
      const left = earCells(folded, false, CX, TOP);
      const right = earCells(folded, true, CX, TOP);

      it('fills the ear with a fixed number of hexes', () => {
        expect(left).toHaveLength(folded ? 22 : 19);
        expect(right).toHaveLength(left.length);
      });

      it('keeps every hex centre inside a facet of the source polygon', () => {
        for (const [cells, mirror] of [[left, false], [right, true]] as const) {
          const facets = earFacets(folded, mirror, CX, TOP);
          cells.forEach((c) => expect(facets.some((f) => insidePolygon(c.x, c.y, f.pts))).toBe(true));
        }
      });

      it('makes the right ear the mirror image of the left ear', () => {
        const key = (c: EarCell) => `${c.x.toFixed(6)},${c.y.toFixed(6)},${c.fill}`;
        const mirrored = new Set(left.map((c) => key({ ...c, x: 2 * CX - c.x })));
        expect(new Set(right.map(key))).toEqual(mirrored);
      });

      it('keeps each ear on its own side, clear of the pin stud in the middle', () => {
        left.forEach((c) => expect(c.x + c.r).toBeLessThan(CX));
        right.forEach((c) => expect(c.x - c.r).toBeGreaterThan(CX));
        // The stud of bubble.ts: a diamond 7 px wide and 9 px tall, with its tip 1.5 px above the top edge.
        const stud: [number, number][] = [[CX, TOP - 1.5], [CX + 3.5, TOP - 6], [CX, TOP - 10.5], [CX - 3.5, TOP - 6]];
        for (const c of [...left, ...right]) {
          const hex = hexCorners(c.r, c.x, c.y);
          stud.forEach(([x, y]) => expect(insidePolygon(x, y, hex)).toBe(false));
          hex.forEach(([x, y]) => expect(insidePolygon(x, y, stud)).toBe(false));
        }
      });

      it('stands every hex at least EAR_LIFT above the head and EAR_DOT_GAP from the next hex', () => {
        const all = [...left, ...right];
        all.forEach((c) => expect(cellGap(c, head)).toBeGreaterThanOrEqual(EAR_LIFT - 1e-9));
        all.forEach((a, i) => all.slice(i + 1).forEach((b) => expect(cellGap(a, b)).toBeGreaterThanOrEqual(EAR_DOT_GAP - 1e-9)));
        all.forEach((c) => expect(c.r).toBe(EAR_DOT_R));
      });
    });
  }

  it('folds lower than a standing ear and hangs the flap further out', () => {
    const up = earCells(false, false, CX, TOP);
    const fold = earCells(true, false, CX, TOP);
    expect(Math.min(...fold.map((c) => c.y))).toBeGreaterThan(Math.min(...up.map((c) => c.y)));
    expect(Math.min(...fold.map((c) => c.x))).toBeLessThan(Math.min(...up.map((c) => c.x)));
  });

  it('marks the kink with the crease tone on a folded ear only', () => {
    expect(earCells(false, false, CX, TOP).some((c) => c.fill === CREASE)).toBe(false);
    expect(earCells(true, false, CX, TOP).filter((c) => c.fill === CREASE).length).toBeGreaterThanOrEqual(2);
  });

  it('keeps the light amethyst inner panel in both shapes', () => {
    for (const folded of [false, true]) {
      expect(earCells(folded, false, CX, TOP).some((c) => c.fill === '#7a4fb8')).toBe(true);
    }
  });
});

describe('ears', () => {
  const count = (state: Parameters<typeof ears>[0]) => ears(state, 0, 0).map((e) => e.length);

  it('folds no ear, the left ear, the right ear or both, by the state', () => {
    expect(count('up')).toEqual([19, 19]);
    expect(count('left')).toEqual([22, 19]);
    expect(count('right')).toEqual([19, 22]);
    expect(count('both')).toEqual([22, 22]);
  });

  it('gives the bounds every hex of every fold state', () => {
    const all = allEarCells(0, 0);
    for (const state of ['up', 'left', 'right', 'both'] as const) {
      ears(state, 0, 0).flat().forEach((c) => expect(all).toContainEqual(c));
    }
  });
});
