import { describe, expect, it } from 'vitest';
import { TEXT_MARGIN, fitText, lineNorms, linesOf } from './fit';
import {
  APOTHEM,
  C,
  OUTLINE_R,
  RING_OUTER,
  type RingGeometry,
  hexPath,
  ringDash,
  ringPair,
  ringsInner
} from './geometry';

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

// All rings of a bubble of `count` accounts, outermost first.
function rings(count: number): RingGeometry[] {
  return Array.from({ length: count }, (_, i) => ringPair(i, count)).flatMap((p) => [p.five, p.seven]);
}

// The apothem of the inner edge of the 1 px outline stroke.
const OUTLINE_INNER = OUTLINE_R * APOTHEM - 0.5;

// The longest synthetic name of the issue, with the widest normal figures and countdown.
const WORST = { top: 'next 5h', centre: '4h 59m', name: 'longname12ch', figures: '99/99%' };

describe('ringPair', () => {
  it('puts a thick 5h ring outside a thin 7d ring', () => {
    for (const count of [1, 2, 3, 4, 5]) {
      const { five, seven } = ringPair(0, count);
      expect(five.sw).toBeGreaterThan(seven.sw);
      expect(seven.outer).toBeLessThan(five.inner);
    }
  });

  it('draws each path on the middle of its stroke', () => {
    const { five } = ringPair(0, 1);
    expect(five).toEqual({ d: hexPath((RING_OUTER - 3) / APOTHEM), sw: 6, outer: RING_OUTER, inner: RING_OUTER - 6 });
    // The top edge of the path sits at the apothem of the stroke middle.
    const y = Number(five.d.split(' ')[2]);
    expect(C - y).toBeCloseTo(RING_OUTER - 3, 6);
  });

  for (const count of [1, 2, 3, 5]) {
    describe(`with ${count} account${count > 1 ? 's' : ''}`, () => {
      const all = rings(count);

      it('keeps every ring inside the hex outline', () => {
        expect(all[0].outer).toBeLessThanOrEqual(OUTLINE_INNER - 1);
      });

      it('keeps a gap of at least 1 px between two rings', () => {
        all.slice(1).forEach((r, i) => expect(all[i].inner - r.outer).toBeGreaterThanOrEqual(1));
      });

      it('keeps strokes of at least 1 px', () => {
        all.forEach((r) => expect(r.sw).toBeGreaterThanOrEqual(1));
      });

      it('keeps the innermost ring outside the centre text', () => {
        const inner = all[all.length - 1].inner;
        expect(ringsInner(count)).toBe(inner);
        const fit = fitText(WORST, inner - TEXT_MARGIN);
        const norms = lineNorms(linesOf(WORST.top, WORST.centre, fit.sub, fit.sizes), fit.offset);
        expect(Math.max(...norms)).toBeLessThanOrEqual(inner - TEXT_MARGIN);
      });
    });
  }

  it('makes the rings thinner as the account count grows', () => {
    const widths = [1, 2, 3, 4, 5].map((n) => ringPair(0, n).five.sw);
    widths.slice(1).forEach((w, i) => expect(w).toBeLessThan(widths[i]));
  });

  it('keeps the ring band of 5 accounts for more than 5 accounts', () => {
    expect(ringsInner(8)).toBeGreaterThanOrEqual(ringsInner(5) - 1);
  });
});

describe('ringDash', () => {
  it('clamps the percent to 0..100', () => {
    expect(ringDash(42)).toBe('42 100');
    expect(ringDash(130)).toBe('100 100');
    expect(ringDash(-5)).toBe('0 100');
  });
});
