import { describe, expect, it } from 'vitest';
import {
  APOTHEM,
  CELL_APOTHEM,
  CELL_R,
  FIVE_RING,
  SEVEN_RING,
  hexCorners,
  hexPath,
  ringDash
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

  it('puts the 7d ring outside the 5h ring, both 3 px wide, with a 1.5 px gap', () => {
    expect(SEVEN_RING.sw).toBe(3);
    expect(FIVE_RING.sw).toBe(3);
    expect(SEVEN_RING.outer).toBeGreaterThan(FIVE_RING.outer);
    expect(SEVEN_RING.inner - FIVE_RING.outer).toBeCloseTo(1.5, 9);
  });

  it('keeps the 7d ring 1.5 px inside the cell outline', () => {
    expect(CELL_APOTHEM - SEVEN_RING.outer).toBeCloseTo(1.5, 9);
  });

  it('draws each ring path on the middle of its stroke', () => {
    for (const g of [FIVE_RING, SEVEN_RING]) {
      expect(g.r * APOTHEM).toBeCloseTo((g.outer + g.inner) / 2, 9);
    }
  });
});

describe('ringDash', () => {
  it('clamps the percent to 0..100', () => {
    expect(ringDash(42)).toBe('42 100');
    expect(ringDash(130)).toBe('100 100');
    expect(ringDash(-5)).toBe('0 100');
  });
});
