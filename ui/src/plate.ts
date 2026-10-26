// The backplate. Issue #21 round 2: on a busy wallpaper the loose cells, ear hexes and pads
// read as noise. One quiet silhouette behind the whole rabbit joins them. The plate is the
// union of a padded copy of every hex, so its outline stays straight-edged and follows the
// honeycomb. The glass fills it, and a faint light line runs along its outside.

import { APOTHEM, type Cell, hexPath } from './geometry';

/** The plate reaches this far past each hex edge, in px. It closes the 3 px gap between cells. */
export const PLATE_PAD = 3;
/** The width of the light line outside the plate, in px. */
export const PLATE_EDGE_W = 1;

/** Returns the padded copy of each hex. The union of the copies is the plate. */
export function plateHexes(hexes: readonly Cell[]): Cell[] {
  return hexes.map((h) => ({ x: h.x, y: h.y, r: h.r + PLATE_PAD / APOTHEM }));
}

/**
 * Returns the plate as one SVG path of hex subpaths. Every subpath runs clockwise, so the
 * nonzero fill rule paints their union, with no seam where two copies overlap.
 */
export function platePath(plate: readonly Cell[]): string {
  return plate.map((h) => hexPath(h.r, h.x, h.y)).join(' ');
}
