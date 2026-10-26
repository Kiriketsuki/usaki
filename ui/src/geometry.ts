// Hex and ring geometry. hexPath and the ring dash come from docs/design/mockup/Bubble.dc.html.
// Issue #11 replaces the one big hex with a cluster of small hex cells, one per account.

/** The ratio of the apothem (centre to edge) to the radius (centre to corner) of a hexagon. */
export const APOTHEM = 0.866;
/**
 * The radius of one cell, centre to corner. A cell is 100 px corner to corner, 87 px flat to
 * flat. A smaller cell cannot hold a 6 character 7d countdown, such as 23h59m, inside the 5h ring.
 */
export const CELL_R = 50;
/** The apothem of one cell. */
export const CELL_APOTHEM = CELL_R * APOTHEM;

/**
 * A flat-top hexagon path around (cx, cy) that starts at the top centre and runs clockwise.
 * The ring progress then grows from 12 o'clock.
 */
export function hexPath(r: number, cx: number, cy: number): string {
  const h = r * APOTHEM;
  return 'M ' + cx + ' ' + (cy - h) +
    ' L ' + (cx + r / 2) + ' ' + (cy - h) +
    ' L ' + (cx + r) + ' ' + cy +
    ' L ' + (cx + r / 2) + ' ' + (cy + h) +
    ' L ' + (cx - r / 2) + ' ' + (cy + h) +
    ' L ' + (cx - r) + ' ' + cy +
    ' L ' + (cx - r / 2) + ' ' + (cy - h) + ' Z';
}

/** The six corners of a flat-top hexagon around (cx, cy), clockwise from the top right. */
export function hexCorners(r: number, cx: number, cy: number): [number, number][] {
  const h = r * APOTHEM;
  return [
    [cx + r / 2, cy - h],
    [cx + r, cy],
    [cx + r / 2, cy + h],
    [cx - r / 2, cy + h],
    [cx - r, cy],
    [cx - r / 2, cy - h]
  ];
}

/**
 * One ring of a cell. `r` is the radius of the path, at the middle of the stroke. `outer`
 * and `inner` are the apothems of the two stroke edges. A stroke grows sw / 2 to each side
 * of the path, at a right angle to each hex side, so the edges are apothems and not radii.
 */
export interface RingGeometry {
  r: number;
  sw: number;
  outer: number;
  inner: number;
}

/** The apothem of the outer edge of the outer ring. It keeps 1.5 px clear of the cell outline. */
export const RING_OUTER = CELL_APOTHEM - 1.5;
/** Both rings have the same stroke, so neither reads as the cell outline. The gap is in px too. */
export const RING_SW = 3;
export const RING_GAP = 1.5;

function ring(outer: number, sw: number): RingGeometry {
  return { r: (outer - sw / 2) / APOTHEM, sw, outer, inner: outer - sw };
}

/** The 7d ring is the outer ring and the 5h ring the inner ring, as the user asked. */
export const SEVEN_RING = ring(RING_OUTER, RING_SW);
export const FIVE_RING = ring(SEVEN_RING.inner - RING_GAP, RING_SW);

/** The stroke-dasharray of a ring with pathLength 100. It clamps pct to 0..100. */
export function ringDash(pct: number): string {
  return Math.max(0, Math.min(100, pct)) + ' 100';
}
