import { describe, expect, it } from 'vitest';
import { C, hexPath, ringDash, ringGeometry } from './geometry';

describe('hexPath', () => {
  it('matches the mockup path for the outline', () => {
    const h = 75 * 0.866;
    expect(hexPath(75)).toBe(
      `M 76 ${76 - h} L 113.5 ${76 - h} L 151 76 L 113.5 ${76 + h} L 38.5 ${76 + h} L 1 76 L 38.5 ${76 - h} Z`
    );
  });

  it('starts at the top centre and runs clockwise', () => {
    const nums = hexPath(40).replace(/[MLZ]/g, ' ').trim().split(/\s+/).map(Number);
    expect(nums[0]).toBe(C);
    expect(nums[1]).toBeLessThan(C);
    expect(nums[2]).toBeGreaterThan(C);
  });
});

describe('ringGeometry', () => {
  it('steps 10 px inward with a 6 px stroke for up to 4 rings', () => {
    expect(ringGeometry(0, 4)).toEqual({ d: hexPath(66), sw: 6 });
    expect(ringGeometry(3, 4)).toEqual({ d: hexPath(36), sw: 6 });
  });

  it('steps 8 px inward with a 5 px stroke for more than 4 rings', () => {
    expect(ringGeometry(1, 5)).toEqual({ d: hexPath(58), sw: 5 });
  });
});

describe('ringDash', () => {
  it('clamps the percent to 0..100', () => {
    expect(ringDash(42)).toBe('42 100');
    expect(ringDash(130)).toBe('100 100');
    expect(ringDash(-5)).toBe('0 100');
  });
});
