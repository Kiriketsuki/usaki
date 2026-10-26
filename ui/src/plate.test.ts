import { describe, expect, it } from 'vitest';
import { ears, insidePolygon } from './ears';
import { APOTHEM, type Cell, cellGap, hexCorners } from './geometry';
import { PAD, layout } from './layout';
import { PLATE_EDGE_W, PLATE_PAD, plateHexes, platePath } from './plate';

const EPS = 1e-6;
const STATES = ['up', 'left', 'right', 'both'] as const;

// The plate of one count and one fold state, as bubble.ts builds it.
function plateOf(count: number, state: (typeof STATES)[number]) {
  const lay = layout(count);
  const head = lay.cells[0];
  const hexes: Cell[] = [...lay.cells, ...ears(state, head.x, lay.earBase).flat(), ...lay.pads];
  return { lay, hexes, plate: plateHexes(hexes) };
}

// Returns true when every hex of the plate links to the first one through overlapping hexes.
function connected(plate: readonly Cell[]): boolean {
  const seen = new Set([0]);
  const queue = [0];
  while (queue.length) {
    const i = queue.pop() as number;
    plate.forEach((h, j) => {
      if (!seen.has(j) && cellGap(plate[i], h) < 0) {
        seen.add(j);
        queue.push(j);
      }
    });
  }
  return seen.size === plate.length;
}

describe('plateHexes', () => {
  it('grows each hex by PLATE_PAD at every edge, around the same centre', () => {
    const [p] = plateHexes([{ x: 10, y: 20, r: 50 }]);
    expect([p.x, p.y]).toEqual([10, 20]);
    expect(p.r * APOTHEM).toBeCloseTo(50 * APOTHEM + PLATE_PAD, 9);
  });

  it('draws one clockwise hex subpath per padded hex', () => {
    const path = platePath(plateHexes([{ x: 0, y: 0, r: 10 }, { x: 30, y: 0, r: 10 }]));
    expect(path.match(/M /g)).toHaveLength(2);
    expect(path.match(/ Z/g)).toHaveLength(2);
  });
});

describe('the plate of a layout', () => {
  for (const count of [1, 2, 3, 4, 5, 6, 8]) {
    for (const state of STATES) {
      describe(`with ${count} accounts and the ears ${state}`, () => {
        const { lay, hexes, plate } = plateOf(count, state);

        it('contains every cell, every ear hex and every foot pad', () => {
          hexes.forEach((h) =>
            hexCorners(h.r, h.x, h.y).forEach(([x, y]) =>
              expect(plate.some((p) => insidePolygon(x, y, hexCorners(p.r + EPS, p.x, p.y)))).toBe(true)
            )
          );
        });

        it('forms one piece, so the rabbit reads as one silhouette', () => {
          expect(connected(plate)).toBe(true);
        });

        it('keeps the plate and its edge line PAD inside the window', () => {
          const margin = PAD + PLATE_EDGE_W - EPS;
          for (const p of plate) {
            expect(p.x - p.r).toBeGreaterThanOrEqual(margin);
            expect(p.x + p.r).toBeLessThanOrEqual(lay.width - margin);
            expect(p.y - p.r * APOTHEM).toBeGreaterThanOrEqual(margin);
            expect(p.y + p.r * APOTHEM).toBeLessThanOrEqual(lay.height - margin);
          }
        });
      });
    }
  }
});
