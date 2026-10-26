import { describe, expect, it } from 'vitest';
import { CELL_GAP, cellGap } from './geometry';
import { PAD, layout } from './layout';
import { PAD_FILL, PAD_R, clearOfCells, footPads } from './pads';

const EPS = 1e-6;

describe('footPads', () => {
  for (const count of [1, 2, 3, 4, 5, 6, 8]) {
    describe(`with ${count} accounts`, () => {
      const lay = layout(count);

      it('draws some pads, all of one size and one surface tone', () => {
        expect(lay.pads.length).toBeGreaterThan(0);
        lay.pads.forEach((p) => expect([p.r, p.fill]).toEqual([PAD_R, PAD_FILL]));
      });

      it('keeps every pad at least CELL_GAP from every cell, so no pad covers a ring', () => {
        lay.pads.forEach((p) => lay.cells.forEach((c) => expect(cellGap(p, c)).toBeGreaterThanOrEqual(CELL_GAP - EPS)));
      });

      it('keeps every pad inside the window with PAD to spare', () => {
        for (const p of lay.pads) {
          expect(p.x - p.r).toBeGreaterThanOrEqual(PAD - EPS);
          expect(p.x + p.r).toBeLessThanOrEqual(lay.width - PAD + EPS);
          expect(p.y + p.r * 0.866).toBeLessThanOrEqual(lay.height - PAD + EPS);
        }
      });

      it('puts the pads in the lower part of the bubble, left and right in balance', () => {
        lay.pads.forEach((p) => expect(p.y).toBeGreaterThan(lay.cells[0].y));
        // Past 6 accounts the body rows fill from the left, so only 1 to 6 balance.
        if (count > 6) return;
        const mid = lay.cells[0].x;
        const key = (x: number, y: number) => `${(x - mid).toFixed(6)},${y.toFixed(6)}`;
        expect(new Set(lay.pads.map((p) => key(2 * mid - p.x, p.y)))).toEqual(new Set(lay.pads.map((p) => key(p.x, p.y))));
      });
    });
  }

  it('hangs two front paws under a head alone', () => {
    const [head] = layout(1).cells;
    const pads = footPads([head]);
    expect(pads).toHaveLength(6);
    pads.forEach((p) => expect(p.y).toBeGreaterThan(head.y + head.r * 0.866));
  });

  it('tucks two front paws under a head between two feet', () => {
    const { cells, pads } = layout(3);
    const [head] = cells;
    const tucked = pads.filter((p) => Math.abs(p.x - head.x) < head.r / 2);
    expect(tucked.length).toBeGreaterThanOrEqual(2);
    tucked.forEach((p) => expect(p.y).toBeGreaterThan(head.y + head.r * 0.866));
  });

  it('drops a pad that would overlap a cell', () => {
    const cell = { x: 0, y: 0, r: 50 };
    const pads = [
      { x: 10, y: 10, r: PAD_R, fill: PAD_FILL },
      { x: 200, y: 0, r: PAD_R, fill: PAD_FILL }
    ];
    expect(clearOfCells(pads, [cell])).toEqual([pads[1]]);
  });
});
