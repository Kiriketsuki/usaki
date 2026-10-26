// The rabbit layout. Issue #21 draws the bubble as a front-view rabbit. The auto account is the
// head: the biggest cell, with the ears on it. The other accounts are smaller body cells below
// the head. Every pair of cells that touch has the same gap, and no two cells overlap. Small
// decor hexes form the ears and the foot pads, and a backplate joins everything. The function
// also gives the window size that holds the plate and its edge line.

import { allEarCells } from './ears';
import { APOTHEM, CELL_APOTHEM, CELL_GAP, CELL_R, type Cell, HEAD_R, cellGap } from './geometry';
import { type Decor, footPads } from './pads';
import { PLATE_EDGE_W, plateHexes } from './plate';

export { CELL_GAP, type Cell, cellGap };
export type { Decor };

export interface Point {
  x: number;
  y: number;
}

// The body cells sit on their own flat-top honeycomb. PITCH_A is the apothem of one step of
// that honeycomb: two body cells one step apart face each other with CELL_GAP between.
const PITCH_A = CELL_APOTHEM + CELL_GAP / 2;
const PITCH_R = PITCH_A / APOTHEM;
const HEAD_A = HEAD_R * APOTHEM;

/** The account count from which the head sits between two shoulder cells. */
export const SHOULDER_COUNT = 6;

/**
 * Returns body site `i`, relative to the chest cell at (0, 0). Site 0 is the chest, under
 * the head. Then come the left haunch, the right haunch and the cell under the chest. More
 * cells repeat that pattern further down, so the body grows as three columns.
 */
export function bodySite(i: number): Point {
  if (i === 0) return { x: 0, y: 0 };
  const row = Math.floor((i - 1) / 3);
  const slot = (i - 1) % 3;
  if (slot === 2) return { x: 0, y: (2 * row + 2) * PITCH_A };
  return { x: (slot === 0 ? -1.5 : 1.5) * PITCH_R, y: (2 * row + 1) * PITCH_A };
}

/**
 * Returns body site `i` of a head between two shoulders, relative to the chest site at
 * (0, 0), which stays empty. Sites 0 and 1 are the left and the right shoulder beside the
 * lower half of the head. Then come the cell under the head and the two haunches. More cells
 * repeat that row of three further down. Six accounts then stay as short as five.
 */
export function shoulderSite(i: number): Point {
  if (i < 2) return { x: (i === 0 ? -1.5 : 1.5) * PITCH_R, y: PITCH_A };
  const row = Math.floor((i - 2) / 3);
  const slot = (i - 2) % 3;
  if (slot === 0) return { x: 0, y: (2 * row + 2) * PITCH_A };
  return { x: (slot === 1 ? -1.5 : 1.5) * PITCH_R, y: (2 * row + 3) * PITCH_A };
}

// Two body cells sit as two feet beside the lower half of the head. Each foot faces a lower
// slanted edge of the head with CELL_GAP between. The head centre then sits at this y,
// relative to the feet sites, which are the left and the right haunch.
function headYOverFeet(): number {
  // The normal of the lower right head edge is (cos 30, sin 30). The foot must sit at
  // HEAD_A + CELL_GAP + CELL_APOTHEM along it from the head centre.
  const reach = HEAD_A + CELL_GAP + CELL_APOTHEM;
  return PITCH_A - 2 * (reach - 1.5 * PITCH_R * APOTHEM);
}

// The cells relative to the chest site, head first. One body cell sits as the chest. Two
// sit as feet. Three to five fill the body sites in order under a head that rests on the
// chest. Six or more put the head between two shoulders and fill the shoulder sites.
function rawCells(count: number): Cell[] {
  const body = Math.max(0, count - 1);
  const bodyCell = (p: Point): Cell => ({ ...p, r: CELL_R });
  if (body === 2) return [{ x: 0, y: headYOverFeet(), r: HEAD_R }, bodyCell(bodySite(1)), bodyCell(bodySite(2))];
  if (count >= SHOULDER_COUNT) {
    return [{ x: 0, y: headYOverFeet(), r: HEAD_R }, ...Array.from({ length: body }, (_, i) => bodyCell(shoulderSite(i)))];
  }
  const head: Cell = { x: 0, y: -(CELL_APOTHEM + CELL_GAP + HEAD_A), r: HEAD_R };
  return [head, ...Array.from({ length: body }, (_, i) => bodyCell(bodySite(i)))];
}

/** The clear space between the plate edge line and the window border, in px. */
export const PAD = 2;

export interface Layout {
  /** Cell centres and radii in window px. Cell 0 is the head. */
  cells: Cell[];
  /** The foot pads in window px. */
  pads: Decor[];
  /** The y of the head top edge in window px. The ears stand on it. */
  earBase: number;
  width: number;
  height: number;
}

// The corners of a flat-top hex as two extents.
function extent(h: Cell): { x0: number; x1: number; y0: number; y1: number } {
  return { x0: h.x - h.r, x1: h.x + h.r, y0: h.y - h.r * APOTHEM, y1: h.y + h.r * APOTHEM };
}

/**
 * Returns the layout of `count` accounts. Zero accounts still get the head cell, for the no
 * data text. The window holds the plate of every fold state, its edge line and PAD on each
 * side, so no pixel at the window border shows. The layout depends on the count only, so a
 * new head or a fold does not change the window size.
 */
export function layout(count: number): Layout {
  const raw = rawCells(Math.max(1, count));
  const head = raw[0];
  const top = head.y - HEAD_A;
  const pads = footPads(raw);
  const all = plateHexes([...raw, ...pads, ...allEarCells(head.x, top)]).map(extent);
  const margin = PAD + PLATE_EDGE_W;
  const dx = margin - Math.min(...all.map((e) => e.x0));
  const dy = margin - Math.min(...all.map((e) => e.y0));
  const shift = <T extends Cell>(c: T): T => ({ ...c, x: c.x + dx, y: c.y + dy });
  return {
    cells: raw.map(shift),
    pads: pads.map(shift),
    earBase: top + dy,
    width: Math.ceil(Math.max(...all.map((e) => e.x1)) + dx + margin),
    height: Math.ceil(Math.max(...all.map((e) => e.y1)) + dy + margin)
  };
}
