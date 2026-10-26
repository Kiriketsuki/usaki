// The rabbit ears, ported from docs/design/mockup/Bubble.dc.html and Main.dc.html.

// The ears sit in a 36 px band above the hex. The hex keeps its 152 x 152 box at top
// 36 px, so the root grows to 152 x 188. EAR_BASE is the hex top edge in the ear SVG:
// the 36 px band plus the 11 px gap between the box top and the edge. The extra 1 px
// keeps a gap from showing under the glass.
const EAR_BASE = 48;
const W = 152;

// Chrysaki surface tones for the facets. The inner facet carries amethyst from the
// tri-primary bar.
const FACET_SHADE = '#1c1f2b';
const FACET_BACK = '#252836';
const FACET_LIT = '#363a4f';
const FACET_INNER = '#583090';

type Point = readonly [number, number];

interface EarShape {
  facets: readonly { pts: readonly Point[]; fill: string }[];
  edge: readonly Point[];
}

// The left ear, standing. Points are [x, y] in the ear SVG. The right ear is the mirror
// image. Every facet is a straight-edged polygon.
const EAR_UP: EarShape = {
  facets: [
    { pts: [[44, 2], [36, 20], [45, EAR_BASE], [56, EAR_BASE], [50, 22]], fill: FACET_BACK },
    { pts: [[44, 2], [50, 22], [56, EAR_BASE], [67, EAR_BASE], [60, 14]], fill: FACET_LIT },
    { pts: [[47, 10], [56, 18], [60, 43], [53, 43], [50, 24]], fill: FACET_INNER }
  ],
  edge: [[45, EAR_BASE], [36, 20], [44, 2], [60, 14], [67, EAR_BASE]]
};

// The left ear, folded. The stem stands to the fold line, and the tip drops outward and
// down beside the hex. The underside of the flap is darker.
const EAR_FOLD: EarShape = {
  facets: [
    { pts: [[38, 26], [45, EAR_BASE], [56, EAR_BASE], [50, 24]], fill: FACET_BACK },
    { pts: [[50, 24], [56, EAR_BASE], [67, EAR_BASE], [61, 21]], fill: FACET_LIT },
    { pts: [[50, 27], [58, 25], [60, 43], [53, 43]], fill: FACET_INNER },
    { pts: [[61, 21], [50, 12], [30, 21], [20, 38], [50, 24]], fill: FACET_LIT },
    { pts: [[50, 24], [20, 38], [38, 26]], fill: FACET_SHADE }
  ],
  edge: [[45, EAR_BASE], [38, 26], [20, 38], [30, 21], [50, 12], [61, 21], [67, EAR_BASE]]
};

export type EarState = 'up' | 'one' | 'both';

export interface EarView {
  facets: { pts: string; fill: string }[];
  edge: string;
}

function pts(list: readonly Point[], mirror: boolean): string {
  return list.map((p) => (mirror ? W - p[0] : p[0]) + ',' + p[1]).join(' ');
}

function ear(shape: EarShape, mirror: boolean): EarView {
  return {
    facets: shape.facets.map((f) => ({ pts: pts(f.pts, mirror), fill: f.fill })),
    edge: pts(shape.edge, mirror)
  };
}

/** Returns the left and the right ear. In the `one` state the right ear folds. */
export function ears(state: EarState): [EarView, EarView] {
  return [
    ear(state === 'both' ? EAR_FOLD : EAR_UP, false),
    ear(state === 'up' ? EAR_UP : EAR_FOLD, true)
  ];
}

/**
 * The fold rule from Main.dc.html. The ears follow the rings of the live accounts. No
 * live account or a full ring folds both ears. A ring at `crit` or more folds one ear.
 */
export function earState(liveRingPcts: readonly number[], crit: number): EarState {
  if (liveRingPcts.length === 0 || liveRingPcts.some((p) => p >= 100)) return 'both';
  if (liveRingPcts.some((p) => p >= crit)) return 'one';
  return 'up';
}
