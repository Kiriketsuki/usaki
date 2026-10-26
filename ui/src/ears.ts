// The rabbit ears. Issue #11 redraws them longer, with a small outward tilt, and with a
// clear kink in the fold. Every shape stays a straight-edged polygon, as Chrysaki asks.

// Chrysaki surface tones for the facets. The inner panel is a lighter amethyst than the
// tri-primary bar, so it reads as the inside of the ear.
const FACET_SHADE = '#1c1f2b';
const FACET_BACK = '#252836';
const FACET_LIT = '#363a4f';
const FACET_INNER = '#7a4fb8';

type Point = readonly [number, number];

interface EarShape {
  facets: readonly { pts: readonly Point[]; fill: string }[];
  edge: readonly Point[];
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
  ],
  edge: [BASE_OUT, KINK_OUT, TIP_OUT, TIP_IN, KINK_IN, BASE_IN]
};

// The folded ear. The lower two thirds stand to a fold line 37 px up. The top third bends
// over the fold line at a sharp kink and hangs outward and down, with a flat-cut tip. The
// ridge of the fold is a back tone and the hanging flap is the darkest tone, so the fold
// reads at a glance.
const FOLD_OUT: Point = [-25.6, -37];
const FOLD_STRIP: Point = [-22.7, -37];
const FOLD_IN: Point = [-8, -37];
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
  ],
  edge: [BASE_OUT, KINK_OUT, FLAP_TIP_IN, FLAP_TIP_OUT, FLAP_BEND, FLAP_RIDGE, FOLD_IN, KINK_IN, BASE_IN]
};

/** The height of the ears above the top edge of the top cell, in px. */
export const EAR_HEIGHT = 56;
/** The widest reach of an ear from the centre of the top cell, in px. The folded flap sets it. */
export const EAR_REACH = 41;

/** Which ears fold. The left ear follows the 7d windows and the right ear the 5h windows. */
export type EarState = 'up' | 'left' | 'right' | 'both';

export interface EarView {
  facets: { pts: string; fill: string }[];
  edge: string;
}

function pts(list: readonly Point[], mirror: boolean, cx: number, baseY: number): string {
  return list.map((p) => (mirror ? cx - p[0] : cx + p[0]) + ',' + (baseY + p[1])).join(' ');
}

function ear(shape: EarShape, mirror: boolean, cx: number, baseY: number): EarView {
  return {
    facets: shape.facets.map((f) => ({ pts: pts(f.pts, mirror, cx, baseY), fill: f.fill })),
    edge: pts(shape.edge, mirror, cx, baseY)
  };
}

/** Returns the left and the right ear, with the base at `baseY` and the middle at `cx`. */
export function ears(state: EarState, cx: number, baseY: number): [EarView, EarView] {
  const leftFolds = state === 'left' || state === 'both';
  const rightFolds = state === 'right' || state === 'both';
  return [
    ear(leftFolds ? EAR_FOLD : EAR_UP, false, cx, baseY),
    ear(rightFolds ? EAR_FOLD : EAR_UP, true, cx, baseY)
  ];
}

/** Every point of both ear shapes, for the bound tests. */
export const EAR_POINTS: readonly Point[] = [EAR_UP, EAR_FOLD].flatMap((s) => [
  ...s.edge,
  ...s.facets.flatMap((f) => f.pts)
]);

/** The 5h and the 7d percent of one live account. */
export interface LiveWindows {
  five: number;
  seven: number;
}

/**
 * The fold rule. Issue #11 split the rule of Main.dc.html between the ears. The right ear
 * folds when a live 5h window is at `crit` or more. The left ear folds when a live 7d
 * window is at `crit` or more. A full window in either set, or no live account, folds both.
 */
export function earState(live: readonly LiveWindows[], crit: number): EarState {
  const full = live.some((w) => w.five >= 100 || w.seven >= 100);
  if (live.length === 0 || full) return 'both';
  const left = live.some((w) => w.seven >= crit);
  const right = live.some((w) => w.five >= crit);
  if (left && right) return 'both';
  if (left) return 'left';
  if (right) return 'right';
  return 'up';
}
