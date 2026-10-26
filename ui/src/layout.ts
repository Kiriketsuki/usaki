// The honeycomb layout: one flat-top hex cell per account, tiled edge to edge into one
// cluster, and the window size that holds the cluster and the ears.

import { CELL_APOTHEM, CELL_R } from './geometry';
import { EAR_HEIGHT, EAR_REACH } from './ears';

/** Axial cell coordinates. q steps right and down, r steps straight down. */
type Axial = readonly [number, number];

// The fill order. Each prefix is a tidy shape: 1 cell, 2 cells on a diagonal, 3 as a
// triangle of cells that all touch, 4 as a diamond, 5 as a kite, 7 as a full flower. Cell 0
// is the top cell, and the ears sit on it.
const FIRST: readonly Axial[] = [[0, 0], [1, 0], [0, 1], [-1, 1], [0, 2], [1, 1], [-1, 2]];
// The flower of 7 has its middle at this cell. More cells fill rings around it.
const MIDDLE: Axial = [0, 1];
const DIRECTIONS: readonly Axial[] = [[1, 0], [0, 1], [-1, 1], [-1, 0], [0, -1], [1, -1]];

// The cells of the hex ring at distance `k` around MIDDLE, clockwise from the top.
function ringCells(k: number): Axial[] {
  const out: Axial[] = [];
  let q = MIDDLE[0];
  let r = MIDDLE[1] - k;
  for (const [dq, dr] of DIRECTIONS) {
    for (let i = 0; i < k; i += 1) {
      out.push([q, r]);
      q += dq;
      r += dr;
    }
  }
  return out;
}

/** Returns the axial coordinates of `count` cells, in fill order. */
export function cellOrder(count: number): Axial[] {
  const order: Axial[] = FIRST.slice(0, count);
  for (let k = 2; order.length < count; k += 1) {
    order.push(...ringCells(k).slice(0, count - order.length));
  }
  return order;
}

export interface Point {
  x: number;
  y: number;
}

/** Returns the centre of an axial cell in px, relative to cell [0, 0]. */
export function axialCentre([q, r]: Axial): Point {
  return { x: 1.5 * CELL_R * q, y: 2 * CELL_APOTHEM * (r + q / 2) };
}

/** The clear space around the cluster and the ears, in px. */
export const PAD = 2;
/** The gap between the ear band top and the tip of the ears, in px. */
const EAR_TOP_GAP = 2;
/** The ear base sits 1 px below the top edge, so no gap shows under the glass. */
const EAR_SINK = 1;

export interface Layout {
  /** Cell centres in window px, in fill order. */
  cells: Point[];
  /** The cell that carries the ears. */
  top: number;
  /** The y of the ear base in window px. */
  earBase: number;
  width: number;
  height: number;
}

// The top cell is the highest one. A tie goes to the cell nearest the middle.
function topCell(cells: readonly Point[]): number {
  return cells.reduce(
    (best, c, i) =>
      c.y < cells[best].y - 1e-9 || (Math.abs(c.y - cells[best].y) < 1e-9 && Math.abs(c.x) < Math.abs(cells[best].x))
        ? i
        : best,
    0
  );
}

/**
 * Returns the layout of `count` cells. Zero accounts still get one cell, for the no data
 * text. The window holds the cluster, the ears above the top cell, and PAD on each side.
 */
export function layout(count: number): Layout {
  const raw = cellOrder(Math.max(1, count)).map(axialCentre);
  const top = topCell(raw);
  const xs = raw.flatMap((c) => [c.x - CELL_R, c.x + CELL_R]);
  const ys = raw.flatMap((c) => [c.y - CELL_APOTHEM, c.y + CELL_APOTHEM]);
  const left = Math.min(...xs, raw[top].x - EAR_REACH);
  const right = Math.max(...xs, raw[top].x + EAR_REACH);
  const earTop = raw[top].y - CELL_APOTHEM - EAR_HEIGHT - EAR_TOP_GAP;
  const dx = PAD - left;
  const dy = PAD - Math.min(earTop, ...ys);
  return {
    cells: raw.map((c) => ({ x: c.x + dx, y: c.y + dy })),
    top,
    earBase: raw[top].y - CELL_APOTHEM + EAR_SINK + dy,
    width: Math.ceil(right + dx + PAD),
    height: Math.ceil(Math.max(...ys) + dy + PAD)
  };
}
