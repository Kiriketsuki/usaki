// The foot pads. Issue #21 draws the bubble as a front-view rabbit. Small flat-top hexes at
// the base of the cluster read as feet. They are decor: no ring, no text, a Chrysaki surface.

import { APOTHEM, CELL_GAP, type Cell, cellGap } from './geometry';

/** One small decor hex: its centre, its corner radius and its fill. */
export interface Decor {
  x: number;
  y: number;
  r: number;
  fill: string;
}

/** The corner radius of one pad hex, in px, and the gap between two pad hexes. */
export const PAD_R = 7;
export const PAD_GAP = 1.5;
/** The lit Chrysaki surface, the tone of the lit ear face. */
export const PAD_FILL = '#363a4f';

// One step of the pad honeycomb.
const PITCH_A = PAD_R * APOTHEM + PAD_GAP / 2;
const PITCH_R = PITCH_A / APOTHEM;
const STEP_X = 1.5 * PITCH_R;

/**
 * The pad of each hind foot starts this far out from the centre of its cell, in px. It is the
 * first column whose lowest hex keeps CELL_GAP from the lower slanted edge of a body cell.
 */
const FOOT_FROM = 39;
/** The columns of each hind foot pad. */
const FOOT_COLUMNS = 3;
/** The front paws of one head sit this far from the middle of the head, in px. */
const PAW_SPREAD = 22;
/** The front paws between two feet sit this far from the middle of the head, in px. */
const TUCKED_PAW_SPREAD = 12;

function hex(x: number, y: number): Decor {
  return { x, y, r: PAD_R, fill: PAD_FILL };
}

// A strip of pad hexes whose lowest hexes sit on `ground`. The strip runs `columns` steps from
// x0 in the direction `side`. Even columns hold two hexes and odd columns hold one, half a
// step up, so the strip is a flat foot two hexes high.
function foot(x0: number, ground: number, side: number, columns: number): Decor[] {
  const low = ground - PAD_R * APOTHEM;
  return Array.from({ length: columns }, (_, c) => {
    const x = x0 + side * c * STEP_X;
    return c % 2 === 0 ? [hex(x, low), hex(x, low - 2 * PITCH_A)] : [hex(x, low - PITCH_A)];
  }).flat();
}

// A column of pad hexes, `width` columns wide in the direction `side`, from the centre y0
// down to the centre y1.
function paw(x0: number, y0: number, y1: number, width: number, side: number): Decor[] {
  const out: Decor[] = [];
  for (let c = 0; c < width; c++) {
    for (let y = y0 + (c % 2) * PITCH_A; y <= y1 + 1e-9; y += 2 * PITCH_A) out.push(hex(x0 + side * c * STEP_X, y));
  }
  return out;
}

/** Drops each pad hex that comes closer than CELL_GAP to a cell, so no pad covers a ring. */
export function clearOfCells(pads: readonly Decor[], cells: readonly Cell[]): Decor[] {
  return pads.filter((p) => cells.every((c) => cellGap(p, c) >= CELL_GAP - 1e-9));
}

/**
 * Returns the foot pads of a cluster, head first. One head alone gets two front paws that
 * hang under it. A body gets a hind foot at each bottom corner, under the outermost lowest
 * cell. A head between two feet also gets two front paws in the space under the head.
 */
export function footPads(cells: readonly Cell[]): Decor[] {
  const [head, ...body] = cells;
  const headBottom = head.y + head.r * APOTHEM;
  const top = headBottom + CELL_GAP + PAD_R * APOTHEM;
  if (body.length === 0) {
    const pads = [-1, 1].flatMap((s) => paw(head.x + s * (PAW_SPREAD - STEP_X / 2), top, top + 2 * PITCH_A, 2, s));
    return clearOfCells(pads, cells);
  }
  const ground = Math.max(...body.map((c) => c.y + c.r * APOTHEM));
  // Each hind foot stands on the bottom edge of its own cell, so it stays beside that cell.
  const pads = [-1, 1].flatMap((s) => {
    const anchor = body.reduce((a, b) => (s * b.x > s * a.x + 1e-9 || (Math.abs(b.x - a.x) < 1e-9 && b.y > a.y) ? b : a));
    return foot(anchor.x + s * FOOT_FROM, anchor.y + anchor.r * APOTHEM, s, FOOT_COLUMNS);
  });
  // A head between two feet has room under it, down to the ground.
  if (headBottom < ground - CELL_GAP && body.every((c) => c.y > head.y && Math.abs(c.x - head.x) > head.r)) {
    pads.push(...[-1, 1].flatMap((s) => paw(head.x + s * TUCKED_PAW_SPREAD, top, ground - PAD_R * APOTHEM, 1, s)));
  }
  return clearOfCells(pads, cells);
}
