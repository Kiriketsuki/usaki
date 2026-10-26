// Hex and ring geometry, ported from docs/design/mockup/Bubble.dc.html.

/** The centre of the 152 x 152 hex box. */
export const C = 76;
/** The radius of the hex outline. */
export const OUTLINE_R = 75;

/**
 * A flat-top hexagon path that starts at the top centre and runs clockwise.
 * The ring progress then grows from 12 o'clock.
 */
export function hexPath(r: number): string {
  const h = r * 0.866;
  return 'M ' + C + ' ' + (C - h) +
    ' L ' + (C + r / 2) + ' ' + (C - h) +
    ' L ' + (C + r) + ' ' + C +
    ' L ' + (C + r / 2) + ' ' + (C + h) +
    ' L ' + (C - r / 2) + ' ' + (C + h) +
    ' L ' + (C - r) + ' ' + C +
    ' L ' + (C - r / 2) + ' ' + (C - h) + ' Z';
}

export interface RingGeometry {
  d: string;
  sw: number;
}

/**
 * Returns the path and stroke width of ring `index` in a bubble of `count` rings.
 * Ring 0 is the outermost. More than 4 rings use a tighter step and a thinner stroke.
 */
export function ringGeometry(index: number, count: number): RingGeometry {
  const dense = count > 4;
  const step = dense ? 8 : 10;
  return { d: hexPath(66 - index * step), sw: dense ? 5 : 6 };
}

/** The stroke-dasharray of a ring with pathLength 100. It clamps pct to 0..100. */
export function ringDash(pct: number): string {
  return Math.max(0, Math.min(100, pct)) + ' 100';
}
