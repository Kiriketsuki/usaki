import { describe, expect, it } from 'vitest';
import {
  APOTHEM,
  CELL_APOTHEM,
  CELL_R,
  FIVE_RING,
  HEAD_R,
  NOTCH_W,
  TRACK_DOT_SPACING,
  FILL_OPACITY,
  cellRing,
  fillLevel,
  interiorRadius,
  hexCorners,
  hexPath,
  ringDash,
  ringNotch,
  trackDots
} from './geometry';

describe('hexPath', () => {
  it('matches the mockup path for the old 152 px outline', () => {
    const h = 75 * 0.866;
    expect(hexPath(75, 76, 76)).toBe(
      `M 76 ${76 - h} L 113.5 ${76 - h} L 151 76 L 113.5 ${76 + h} L 38.5 ${76 + h} L 1 76 L 38.5 ${76 - h} Z`
    );
  });

  it('starts at the top centre and runs clockwise', () => {
    const nums = hexPath(40, 50, 60).replace(/[MLZ]/g, ' ').trim().split(/\s+/).map(Number);
    expect(nums[0]).toBe(50);
    expect(nums[1]).toBeLessThan(60);
    expect(nums[2]).toBeGreaterThan(50);
  });

  it('passes through the corners of hexCorners', () => {
    const nums = hexPath(CELL_R, 10, 20).replace(/[MLZ]/g, ' ').trim().split(/\s+/).map(Number);
    const corners = hexCorners(CELL_R, 10, 20);
    corners.forEach(([x, y], i) => {
      expect(nums[2 + i * 2]).toBeCloseTo(x, 9);
      expect(nums[3 + i * 2]).toBeCloseTo(y, 9);
    });
  });
});

describe('cell rings', () => {
  it('sizes a cell 100 px corner to corner', () => {
    expect(2 * CELL_R).toBe(100);
    expect(2 * CELL_APOTHEM).toBeCloseTo(86.6, 1);
  });

  it('draws one 3 px ring, 1.5 px inside the cell outline', () => {
    expect(FIVE_RING.sw).toBe(3);
    expect(CELL_APOTHEM - FIVE_RING.outer).toBeCloseTo(1.5, 9);
    expect(FIVE_RING.outer - FIVE_RING.inner).toBeCloseTo(3, 9);
  });

  it('draws the ring path on the middle of its stroke', () => {
    expect(FIVE_RING.r * APOTHEM).toBeCloseTo((FIVE_RING.outer + FIVE_RING.inner) / 2, 9);
  });

  it('scales the ring with the cell', () => {
    expect(cellRing(HEAD_R).sw).toBeCloseTo(3 * (HEAD_R / CELL_R), 9);
  });
});

describe('fillLevel', () => {
  const g = FIVE_RING;
  const height = (pct: number) => fillLevel(g, pct, 100).height;

  it('fills nothing at 0 and the whole interior at 100', () => {
    expect(height(0)).toBe(0);
    expect(height(100)).toBeCloseTo(2 * g.inner, 9);
  });

  it('fills half the interior height at 50, from the bottom up', () => {
    expect(height(50)).toBeCloseTo(g.inner, 9);
    expect(fillLevel(g, 50, 100).y).toBeCloseTo(100, 9);
  });

  it('clamps a percent over 100 and under 0', () => {
    expect(height(130)).toBeCloseTo(2 * g.inner, 9);
    expect(fillLevel(g, 130, 100).y).toBeCloseTo(100 - g.inner, 9);
    expect(height(-5)).toBe(0);
  });

  it('keeps the interior hex inside the ring and the fill body at a low opacity', () => {
    expect(interiorRadius(g) * APOTHEM).toBeCloseTo(g.inner, 9);
    expect(FILL_OPACITY).toBeGreaterThanOrEqual(0.28);
    expect(FILL_OPACITY).toBeLessThanOrEqual(0.35);
  });
});

describe('ringNotch', () => {
  const parse = (s: string) => s.split(' ').map((p) => p.split(',').map(Number));

  it('cuts across the whole ring at 12 o clock, with 0.5 px to spare on each edge', () => {
    const pts = parse(ringNotch(FIVE_RING, CELL_R, 100, 200));
    const xs = pts.map(([x]) => x);
    const ys = pts.map(([, y]) => y);
    expect(Math.max(...xs) - Math.min(...xs)).toBeCloseTo(NOTCH_W, 9);
    expect((Math.max(...xs) + Math.min(...xs)) / 2).toBeCloseTo(100, 9);
    expect(Math.min(...ys)).toBeCloseTo(200 - FIVE_RING.outer - 0.5, 9);
    expect(Math.max(...ys)).toBeCloseTo(200 - FIVE_RING.inner + 0.5, 9);
  });

  it('stays inside the cell', () => {
    const ys = parse(ringNotch(FIVE_RING, CELL_R, 0, 0)).map(([, y]) => y);
    expect(-Math.min(...ys)).toBeLessThan(CELL_APOTHEM);
  });

  it('grows with the cell', () => {
    const big = cellRing(HEAD_R);
    const xs = parse(ringNotch(big, HEAD_R, 0, 0)).map(([x]) => x);
    expect(Math.max(...xs) - Math.min(...xs)).toBeCloseTo((NOTCH_W * HEAD_R) / CELL_R, 9);
  });
});

describe('ringDash', () => {
  it('clamps the percent to 0..100', () => {
    expect(ringDash(42)).toBe('42 100');
    expect(ringDash(130)).toBe('100 100');
    expect(ringDash(-5)).toBe('0 100');
  });
});

describe('trackDots', () => {
  const five = cellRing(CELL_R);
  const dots = trackDots(five.r, 100, 100, TRACK_DOT_SPACING);
  const dist = (a: readonly number[], b: readonly number[]) => Math.hypot(a[0] - b[0], a[1] - b[1]);

  it('puts 12 dots per corner pair, so the count is a multiple of 12', () => {
    expect(dots.length % 12).toBe(0);
    expect(dots.length).toBe(36);
  });

  it('starts at 12 o clock and steps evenly along the path, close to the target spacing', () => {
    expect(dots[0][0]).toBeCloseTo(100, 6);
    expect(dots[0][1]).toBeCloseTo(100 - five.r * APOTHEM, 6);
    const step = (6 * five.r) / dots.length;
    expect(Math.abs(step - TRACK_DOT_SPACING)).toBeLessThan(TRACK_DOT_SPACING / 2);
    // On one side two neighbours are one step apart. Across a corner the chord is shorter.
    dots.forEach((d, i) => {
      const gap = dist(d, dots[(i + 1) % dots.length]);
      expect(gap).toBeLessThanOrEqual(step + 1e-6);
      expect(gap).toBeGreaterThan(step * 0.85);
    });
  });

  it('puts a dot on every corner', () => {
    for (const corner of hexCorners(five.r, 100, 100)) {
      expect(Math.min(...dots.map((d) => dist(d, corner)))).toBeLessThan(1e-6);
    }
  });

  it('keeps every dot on the track path', () => {
    for (const [x, y] of dots) {
      // A point on a flat-top hex path has a hex norm equal to the apothem.
      const dx = Math.abs(x - 100);
      const dy = Math.abs(y - 100);
      expect(Math.max(dy, dx * APOTHEM + dy / 2)).toBeCloseTo(five.r * APOTHEM, 2);
    }
  });

  it('scales with the cell, so the head track has the same count', () => {
    const head = cellRing(HEAD_R);
    expect(trackDots(head.r, 0, 0, (TRACK_DOT_SPACING * HEAD_R) / CELL_R)).toHaveLength(dots.length);
  });
});
