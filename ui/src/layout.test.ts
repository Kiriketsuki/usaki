import { describe, expect, it } from 'vitest';
import { EAR_HEIGHT, EAR_POINTS } from './ears';
import { CELL_APOTHEM, CELL_R, hexCorners } from './geometry';
import { PAD, axialCentre, cellOrder, layout } from './layout';

const STEP = 2 * CELL_APOTHEM;
// APOTHEM is 0.866, not the exact root 3 / 2, so a diagonal neighbour sits up to 0.002 px
// further than STEP. The corners still meet exactly, and the tests check the corners.
const SLACK = 0.01;
const dist = (a: { x: number; y: number }, b: { x: number; y: number }) => Math.hypot(a.x - b.x, a.y - b.y);

// The corners that two cells share.
function sharedCorners(a: { x: number; y: number }, b: { x: number; y: number }): number {
  const ca = hexCorners(CELL_R, a.x, a.y);
  const cb = hexCorners(CELL_R, b.x, b.y);
  return ca.filter(([x, y]) => cb.some(([u, v]) => Math.abs(x - u) < 1e-6 && Math.abs(y - v) < 1e-6)).length;
}

describe('cellOrder', () => {
  it('gives tidy shapes for 1 to 5 cells', () => {
    expect(cellOrder(5)).toEqual([[0, 0], [1, 0], [0, 1], [-1, 1], [0, 2]]);
  });

  it('keeps general beyond the first 7 cells and never repeats a cell', () => {
    const cells = cellOrder(19).map((c) => c.join(','));
    expect(new Set(cells).size).toBe(19);
  });
});

describe('layout', () => {
  for (const count of [1, 2, 3, 4, 5, 7, 8, 12]) {
    describe(`with ${count} cells`, () => {
      const lay = layout(count);

      it('never overlaps two cells', () => {
        lay.cells.forEach((a, i) =>
          lay.cells.slice(i + 1).forEach((b) => expect(dist(a, b)).toBeGreaterThan(STEP - SLACK))
        );
      });

      it('joins every cell to an earlier cell by a whole shared edge', () => {
        lay.cells.slice(1).forEach((c, i) => {
          const near = lay.cells.slice(0, i + 1).filter((p) => dist(p, c) < STEP + SLACK);
          expect(near.length).toBeGreaterThan(0);
          near.forEach((p) => expect(sharedCorners(p, c)).toBe(2));
        });
      });

      it('keeps every cell inside the window with PAD to spare', () => {
        for (const c of lay.cells) {
          expect(c.x - CELL_R).toBeGreaterThanOrEqual(PAD - 1e-9);
          expect(c.x + CELL_R).toBeLessThanOrEqual(lay.width - PAD + 1e-9);
          expect(c.y + CELL_APOTHEM).toBeLessThanOrEqual(lay.height - PAD + 1e-9);
        }
      });

      it('puts the ears on the top edge of the highest cell, inside the window', () => {
        const top = lay.cells[lay.top];
        lay.cells.forEach((c) => expect(c.y).toBeGreaterThanOrEqual(top.y - 1e-9));
        expect(lay.earBase).toBeCloseTo(top.y - CELL_APOTHEM + 1, 9);
        for (const [x, y] of EAR_POINTS) {
          for (const px of [top.x + x, top.x - x]) {
            expect(px).toBeGreaterThanOrEqual(0);
            expect(px).toBeLessThanOrEqual(lay.width);
          }
          expect(lay.earBase + y).toBeGreaterThanOrEqual(0);
        }
        expect(lay.earBase - EAR_HEIGHT).toBeGreaterThanOrEqual(PAD);
      });
    });
  }

  it('sizes the one cell window to match the Rust start size', () => {
    expect([layout(1).width, layout(1).height]).toEqual([92, 139]);
  });

  it('gives zero accounts the one cell window', () => {
    expect(layout(0)).toEqual(layout(1));
  });

  it('makes 3 cells a triangle where every cell touches the other two', () => {
    const [a, b, c] = layout(3).cells;
    expect([sharedCorners(a, b), sharedCorners(b, c), sharedCorners(a, c)]).toEqual([2, 2, 2]);
  });

  it('makes 4 cells a diamond with the top and the bottom cell on one vertical line', () => {
    const [top, right, bottom, left] = layout(4).cells;
    expect(bottom.x).toBeCloseTo(top.x, 9);
    expect(bottom.y - top.y).toBeCloseTo(STEP, 9);
    expect(right.x - top.x).toBeCloseTo(top.x - left.x, 9);
    expect(right.y).toBeCloseTo(left.y, 9);
  });

  it('grows the window with the cell count', () => {
    const sizes = [1, 2, 4, 5].map((n) => layout(n));
    sizes.slice(1).forEach((s, i) => expect(s.width * s.height).toBeGreaterThan(sizes[i].width * sizes[i].height));
  });

  it('places cells on the flat-top lattice', () => {
    expect(axialCentre([1, 0])).toEqual({ x: 1.5 * CELL_R, y: CELL_APOTHEM });
    expect(axialCentre([0, 1])).toEqual({ x: 0, y: 2 * CELL_APOTHEM });
  });
});
