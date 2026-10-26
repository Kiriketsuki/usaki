import { describe, expect, it } from 'vitest';
import { ears } from './ears';
import { APOTHEM, CELL_R, HEAD_R } from './geometry';
import { CELL_GAP, PAD, SHOULDER_COUNT, bodySite, cellGap, layout, shoulderSite } from './layout';

const EPS = 1e-6;

describe('layout', () => {
  for (const count of [1, 2, 3, 4, 5, 6, 8]) {
    describe(`with ${count} accounts`, () => {
      const lay = layout(count);
      const [head, ...body] = lay.cells;

      it('makes the head the one biggest cell and every other cell a body cell', () => {
        expect(head.r).toBe(HEAD_R);
        body.forEach((c) => expect(c.r).toBe(CELL_R));
        expect(HEAD_R).toBeGreaterThan(CELL_R);
      });

      it('never overlaps two cells and keeps at least CELL_GAP between any two', () => {
        lay.cells.forEach((a, i) =>
          lay.cells.slice(i + 1).forEach((b) => expect(cellGap(a, b)).toBeGreaterThan(CELL_GAP - EPS))
        );
      });

      it('joins every body cell to the cluster at exactly CELL_GAP', () => {
        body.forEach((c, i) => {
          const before = lay.cells.slice(0, i + 1);
          expect(before.some((p) => Math.abs(cellGap(p, c) - CELL_GAP) < EPS)).toBe(true);
        });
      });

      it('puts the head on top and the body below it', () => {
        body.forEach((c) => expect(c.y).toBeGreaterThan(head.y));
        lay.cells.forEach((c) => expect(c.y - c.r * APOTHEM).toBeGreaterThanOrEqual(head.y - head.r * APOTHEM - EPS));
      });

      it('keeps every cell inside the window with PAD to spare', () => {
        for (const c of lay.cells) {
          expect(c.x - c.r).toBeGreaterThanOrEqual(PAD - EPS);
          expect(c.x + c.r).toBeLessThanOrEqual(lay.width - PAD + EPS);
          expect(c.y + c.r * APOTHEM).toBeLessThanOrEqual(lay.height - PAD + EPS);
        }
      });

      it('stands the ears of every fold state on the head, inside the window', () => {
        expect(lay.earBase).toBeCloseTo(head.y - head.r * APOTHEM, 9);
        for (const state of ['up', 'left', 'right', 'both'] as const) {
          for (const c of ears(state, head.x, lay.earBase).flat()) {
            expect(c.x - c.r).toBeGreaterThanOrEqual(PAD - EPS);
            expect(c.x + c.r).toBeLessThanOrEqual(lay.width - PAD + EPS);
            expect(c.y - c.r * APOTHEM).toBeGreaterThanOrEqual(PAD - EPS);
            expect(c.y).toBeLessThan(lay.earBase);
          }
        }
      });
    });
  }

  for (const count of [2, 3, 4, 5, 6]) {
    it(`balances ${count} accounts left and right around the head`, () => {
      const { cells } = layout(count);
      const mid = cells[0].x;
      const key = (x: number, y: number) => `${(x - mid).toFixed(6)},${y.toFixed(6)}`;
      const here = new Set(cells.map((c) => key(c.x, c.y)));
      const mirrored = new Set(cells.map((c) => key(2 * mid - c.x, c.y)));
      expect(mirrored).toEqual(here);
    });
  }

  it('sets the head on the chest for 2 accounts', () => {
    const [head, chest] = layout(2).cells;
    expect(chest.x).toBeCloseTo(head.x, 9);
    expect(cellGap(head, chest)).toBeCloseTo(CELL_GAP, 9);
  });

  it('puts 2 body cells as feet against the lower slanted edges of the head', () => {
    const [head, left, right] = layout(3).cells;
    expect(cellGap(head, left)).toBeCloseTo(CELL_GAP, 9);
    expect(cellGap(head, right)).toBeCloseTo(CELL_GAP, 9);
    expect(left.y).toBeCloseTo(right.y, 9);
    expect(left.y).toBeGreaterThan(head.y);
  });

  it('builds 3 body cells as a chest and two haunches, and 4 adds a cell under the chest', () => {
    const [, chest4, left, right] = layout(4).cells;
    expect(cellGap(chest4, left)).toBeCloseTo(CELL_GAP, 9);
    expect(cellGap(chest4, right)).toBeCloseTo(CELL_GAP, 9);
    const [, chest, , , under] = layout(5).cells;
    expect(under.x).toBeCloseTo(chest.x, 9);
    expect(under.y).toBeGreaterThan(chest.y);
  });

  it('sizes the window for each count from 1 to 6', () => {
    const sizes = [1, 2, 3, 4, 5, 6].map((n) => [layout(n).width, layout(n).height]);
    expect(sizes).toEqual([[134, 233], [144, 293], [299, 257], [299, 338], [299, 383], [299, 347]]);
  });

  it('sizes the one cell window to match the Rust start size', () => {
    expect([layout(1).width, layout(1).height]).toEqual([134, 233]);
  });

  it('keeps 6 accounts under 360 px tall, with the head between two shoulders', () => {
    const lay = layout(SHOULDER_COUNT);
    expect(lay.height).toBeLessThanOrEqual(360);
    const [head, left, right, under, ...haunches] = lay.cells;
    expect(cellGap(head, left)).toBeCloseTo(CELL_GAP, 9);
    expect(cellGap(head, right)).toBeCloseTo(CELL_GAP, 9);
    expect(under.x).toBeCloseTo(head.x, 9);
    expect(under.y).toBeGreaterThan(head.y);
    haunches.forEach((c) => expect(c.y).toBeGreaterThan(under.y));
  });

  it('grows the shoulder body as rows of three under the head', () => {
    expect(shoulderSite(2).x).toBeCloseTo(0, 9);
    expect(shoulderSite(3).x).toBeCloseTo(shoulderSite(0).x, 9);
    expect(shoulderSite(5).y).toBeGreaterThan(shoulderSite(2).y);
  });

  it('gives zero accounts the head only window', () => {
    expect(layout(0)).toEqual(layout(1));
  });

  it('grows the body as three columns past 5 accounts', () => {
    expect(bodySite(4).x).toBeCloseTo(bodySite(1).x, 9);
    expect(bodySite(6).x).toBeCloseTo(0, 9);
    expect(bodySite(7).y).toBeGreaterThan(bodySite(4).y);
  });
});
