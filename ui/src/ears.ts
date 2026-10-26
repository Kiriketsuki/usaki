// The rabbit ears. Issue #11 drew them as faceted polygons. Issue #21 keeps those polygons as
// the source shape and fills them with small flat-top hexes, so hexagons form the whole
// rabbit. Each small hex takes the tone of the facet that holds its centre.

import { APOTHEM } from './geometry';

// Chrysaki surface tones for the facets. The inner panel is a lighter amethyst than the
// tri-primary bar, so it reads as the inside of the ear.
const FACET_SHADE = '#1c1f2b';
const FACET_BACK = '#252836';
const FACET_LIT = '#363a4f';
const FACET_INNER = '#7a4fb8';

type Point = readonly [number, number];

interface EarShape {
  facets: readonly { pts: readonly Point[]; fill: string }[];
}

// Each point holds an x and a y in px. The x axis starts at the centre of the top cell, and
// the left ear has negative x. The y axis starts at the top edge of the top cell and grows
// down, so the ear has negative y. The right ear is the mirror image.
//
// The standing ear has a 17 px base on the top edge, from x -21 to x -4. It widens a
// little to the kinks, then narrows to a flat-cut tip 56 px up and 4 px out from the
// middle of the base. A flat tip reads softer than a point, with no curve. A narrow dark
// strip runs up the outer edge. The lit inner face holds a leaf of light amethyst.
const BASE_OUT: Point = [-21, 0];
const BASE_IN: Point = [-4, 0];
const KINK_OUT: Point = [-27, -30];
const KINK_IN: Point = [-6, -33];
const TIP_OUT: Point = [-22, -55];
const TIP_IN: Point = [-17, -56];
const STRIP_BASE: Point = [-18, 0];
const STRIP_KINK: Point = [-23, -30];

const EAR_UP: EarShape = {
  facets: [
    { pts: [BASE_OUT, KINK_OUT, TIP_OUT, STRIP_KINK, STRIP_BASE], fill: FACET_BACK },
    { pts: [STRIP_BASE, STRIP_KINK, TIP_OUT, TIP_IN, KINK_IN, BASE_IN], fill: FACET_LIT },
    { pts: [[-15, -5], [-20, -30], [-19.5, -48], [-15.5, -46], [-9, -30], [-8, -5]], fill: FACET_INNER }
  ]
};

// The folded ear. The lower two thirds stand to a fold line 37 px up. The top third bends
// over the fold line at a sharp kink and hangs outward and down, with a flat-cut tip. The
// ridge of the fold is a back tone and the hanging flap is the darkest tone, so the fold
// reads at a glance.
/** The y of the fold line above the top edge, in body cell px. */
const FOLD_Y = -37;
const FOLD_OUT: Point = [-25.6, FOLD_Y];
const FOLD_STRIP: Point = [-22.7, FOLD_Y];
const FOLD_IN: Point = [-8, FOLD_Y];
const FLAP_RIDGE: Point = [-13, -44];
const FLAP_BEND: Point = [-31, -40];
const FLAP_TIP_OUT: Point = [-41, -25];
const FLAP_TIP_IN: Point = [-36, -21];

const EAR_FOLD: EarShape = {
  facets: [
    { pts: [BASE_OUT, KINK_OUT, FOLD_OUT, FOLD_STRIP, STRIP_KINK, STRIP_BASE], fill: FACET_BACK },
    { pts: [STRIP_BASE, STRIP_KINK, FOLD_STRIP, FOLD_IN, KINK_IN, BASE_IN], fill: FACET_LIT },
    { pts: [[-15, -5], [-20, -30], [-17, -35], [-9, -30], [-8, -5]], fill: FACET_INNER },
    { pts: [FOLD_IN, FLAP_RIDGE, FLAP_BEND, FOLD_OUT], fill: FACET_BACK },
    { pts: [FOLD_OUT, FLAP_BEND, FLAP_TIP_OUT, FLAP_TIP_IN, KINK_OUT], fill: FACET_SHADE }
  ]
};

/**
 * The ear points above fit a body cell. The mosaic ears scale them by EAR_SCALE_X across and
 * by EAR_SCALE_Y up. An ear hex is 10 px across, so a narrower ear holds too few hexes to
 * show its facets, and a longer ear reads more as a rabbit ear.
 */
export const EAR_SCALE_X = 1.4;
export const EAR_SCALE_Y = 1.8;
/** Each ear leans out by this many px per px of height, so the two ears open into a V. */
export const EAR_LEAN = 0.16;

/** The corner radius of one ear hex, in px, and the gap between two ear hexes. */
export const EAR_DOT_R = 5;
export const EAR_DOT_GAP = 1.2;
// One step of the ear honeycomb. Two ear hexes one step apart face each other with EAR_DOT_GAP.
const DOT_PITCH_A = EAR_DOT_R * APOTHEM + EAR_DOT_GAP / 2;
const DOT_PITCH_R = DOT_PITCH_A / APOTHEM;
/** The first three facets of a folded ear stand below the fold line. The others bend over it. */
const STANDING_FACETS = 3;
/** The ridge row is the standing hexes within this many px under the fold line: one row. */
const RIDGE_DEPTH = 2 * DOT_PITCH_A;
/** The gap between the lowest ear hexes and the top edge of the head, in px. It is the cell gap. */
export const EAR_LIFT = 3;

/** Which ears fold. The left ear follows the 7d windows and the right ear the 5h windows. */
export type EarState = 'up' | 'left' | 'right' | 'both';

/** One small hex of an ear: its centre, its corner radius and its fill. */
export interface EarCell {
  x: number;
  y: number;
  r: number;
  fill: string;
}

/** A facet in px: its corner points and its fill. */
export interface EarFacet {
  pts: [number, number][];
  fill: string;
}

/**
 * Returns the facets of one ear in px. `cx` is the middle of the head and `top` the y of its
 * top edge. The left ear has negative x and the right ear is its mirror image.
 */
export function earFacets(folded: boolean, mirror: boolean, cx: number, top: number): EarFacet[] {
  const s = mirror ? -1 : 1;
  return (folded ? EAR_FOLD : EAR_UP).facets.map((f) => ({
    pts: f.pts.map(([px, py]) => [cx + s * (px * EAR_SCALE_X + EAR_LEAN * py * EAR_SCALE_Y), top + py * EAR_SCALE_Y]),
    fill: f.fill
  }));
}

/** Returns true when (x, y) lies inside the polygon `pts`, by the even-odd rule. */
export function insidePolygon(x: number, y: number, pts: readonly (readonly [number, number])[]): boolean {
  let inside = false;
  for (let i = 0, j = pts.length - 1; i < pts.length; j = i++) {
    const [xi, yi] = pts[i];
    const [xj, yj] = pts[j];
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) inside = !inside;
  }
  return inside;
}

/**
 * Fills one ear with small hexes. The ear honeycomb has its lowest row EAR_LIFT above the head
 * top edge, and its middle column on the middle of the head. The right ear is then the exact
 * mirror image of the left ear. The fill keeps a hex when its centre lies inside a facet. The
 * hex takes the fill of the last facet that holds it, as the polygons paint in that order.
 */
export function earCells(folded: boolean, mirror: boolean, cx: number, top: number): EarCell[] {
  const facets = earFacets(folded, mirror, cx, top);
  const foldY = top + FOLD_Y * EAR_SCALE_Y;
  const base = top - EAR_LIFT - EAR_DOT_R * APOTHEM;
  const xs = facets.flatMap((f) => f.pts.map((p) => p[0]));
  const ys = facets.flatMap((f) => f.pts.map((p) => p[1]));
  const qMin = Math.floor((Math.min(...xs) - cx) / (1.5 * DOT_PITCH_R)) - 1;
  const qMax = Math.ceil((Math.max(...xs) - cx) / (1.5 * DOT_PITCH_R)) + 1;
  const rows = Math.ceil((base - Math.min(...ys)) / (2 * DOT_PITCH_A)) + 2;
  const out: EarCell[] = [];
  for (let q = qMin; q <= qMax; q++) {
    for (let k = -1; k <= rows; k++) {
      // Column q starts half a step higher when q is odd, so the rows of the honeycomb interlock.
      const x = cx + 1.5 * DOT_PITCH_R * q;
      const y = base - 2 * DOT_PITCH_A * k - (Math.abs(q) % 2) * DOT_PITCH_A;
      if (y > base + 1e-9) continue;
      const inFacets = facets.map((f) => insidePolygon(x, y, f.pts));
      const last = inFacets.lastIndexOf(true);
      if (last < 0) continue;
      // The top row of the standing part of a folded ear is the ridge of the kink. It takes the
      // darkest tone, so the crease shows at 1x between the standing part and the flap.
      const ridge = folded && last < STANDING_FACETS && y < foldY + RIDGE_DEPTH;
      out.push({ x, y, r: EAR_DOT_R, fill: ridge ? FACET_SHADE : facets[last].fill });
    }
  }
  return out;
}

/** Returns the small hexes of the left and the right ear for a fold state. */
export function ears(state: EarState, cx: number, top: number): [EarCell[], EarCell[]] {
  const leftFolds = state === 'left' || state === 'both';
  const rightFolds = state === 'right' || state === 'both';
  return [earCells(leftFolds, false, cx, top), earCells(rightFolds, true, cx, top)];
}

/** Every ear hex of every fold state, for the window bounds. The bounds then never change with a fold. */
export function allEarCells(cx: number, top: number): EarCell[] {
  return [...ears('up', cx, top).flat(), ...ears('both', cx, top).flat()];
}

/** The 5h and the 7d percent of one live account. A 7d-only account has no 5h percent. */
export interface LiveWindows {
  five: number | null;
  seven: number;
}

/**
 * The fold rule. Issue #11 split the rule of Main.dc.html between the ears. The right ear
 * folds when a live 5h window is at `crit` or more. The left ear folds when a live 7d
 * window is at `crit` or more. A full window in either set, or no live account, folds both.
 * A 7d-only account counts for the left ear only.
 */
export function earState(live: readonly LiveWindows[], crit: number): EarState {
  const five = (w: LiveWindows, at: number) => w.five != null && w.five >= at;
  const full = live.some((w) => five(w, 100) || w.seven >= 100);
  if (live.length === 0 || full) return 'both';
  const left = live.some((w) => w.seven >= crit);
  const right = live.some((w) => five(w, crit));
  if (left && right) return 'both';
  if (left) return 'left';
  if (right) return 'right';
  return 'up';
}
