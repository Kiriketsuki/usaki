// Hex and ring geometry, ported from docs/design/mockup/Bubble.dc.html. Issue #11 replaces
// the one ring per account of the mockup with a pair of rings per account.

/** The centre of the 152 x 152 hex box. */
export const C = 76;
/** The radius of the hex outline. */
export const OUTLINE_R = 75;
/** The ratio of the apothem (centre to edge) to the radius (centre to corner) of a hexagon. */
export const APOTHEM = 0.866;
/** The apothem of the outer edge of the outermost ring. The outline sits at 65. */
export const RING_OUTER = 63;

/**
 * A flat-top hexagon path that starts at the top centre and runs clockwise.
 * The ring progress then grows from 12 o'clock.
 */
export function hexPath(r: number): string {
  const h = r * APOTHEM;
  return 'M ' + C + ' ' + (C - h) +
    ' L ' + (C + r / 2) + ' ' + (C - h) +
    ' L ' + (C + r) + ' ' + C +
    ' L ' + (C + r / 2) + ' ' + (C + h) +
    ' L ' + (C - r / 2) + ' ' + (C + h) +
    ' L ' + (C - r) + ' ' + C +
    ' L ' + (C - r / 2) + ' ' + (C - h) + ' Z';
}

/**
 * One ring. `d` and `sw` go to the SVG path. `outer` and `inner` are the apothems of the
 * two stroke edges. A stroke grows sw / 2 to each side of the path, at a right angle to
 * each hex side, so the edges are apothems and not radii.
 */
export interface RingGeometry {
  d: string;
  sw: number;
  outer: number;
  inner: number;
}

/** The 5h ring outside and the 7d ring just inside it, for one account. */
export interface RingPair {
  five: RingGeometry;
  seven: RingGeometry;
}

/** Stroke widths and gaps of a ring pair, in px. */
interface PairSpec {
  /** The stroke of the 5h ring. */
  thick: number;
  /** The stroke of the 7d ring. */
  thin: number;
  /** The gap between the 5h ring and the 7d ring of one account. */
  gap: number;
  /** The gap between two accounts. It is wider than `gap`, so the pairs read as pairs. */
  pairGap: number;
}

// One spec for each account count from 1 to 5. More accounts get thinner strokes and
// tighter gaps, so the innermost ring keeps clear of the centre text.
const PAIR_SPECS: readonly PairSpec[] = [
  { thick: 6, thin: 3, gap: 1.5, pairGap: 3 },
  { thick: 5, thin: 2.5, gap: 1.5, pairGap: 2.5 },
  { thick: 4, thin: 2, gap: 1, pairGap: 2 },
  { thick: 3, thin: 1.5, gap: 1, pairGap: 1.5 },
  { thick: 2.5, thin: 1, gap: 1, pairGap: 1.5 }
];

// More than 5 accounts scale the 5 account spec down, so the rings take no more room.
function pairSpec(count: number): PairSpec {
  const n = Math.max(1, count);
  if (n <= PAIR_SPECS.length) return PAIR_SPECS[n - 1];
  const last = PAIR_SPECS[PAIR_SPECS.length - 1];
  const k = PAIR_SPECS.length / n;
  return { thick: last.thick * k, thin: last.thin * k, gap: last.gap * k, pairGap: last.pairGap * k };
}

function ring(outer: number, sw: number): RingGeometry {
  const mid = outer - sw / 2;
  return { d: hexPath(mid / APOTHEM), sw, outer, inner: outer - sw };
}

/**
 * Returns the two rings of account `index` in a bubble of `count` accounts.
 * Account 0 is the outermost pair.
 */
export function ringPair(index: number, count: number): RingPair {
  const s = pairSpec(count);
  const pitch = s.thick + s.gap + s.thin + s.pairGap;
  const five = ring(RING_OUTER - index * pitch, s.thick);
  const seven = ring(five.inner - s.gap, s.thin);
  return { five, seven };
}

/**
 * Returns the apothem of the inner edge of the innermost ring of `count` accounts. The
 * centre text must stay inside it. With no account, the text can use the full ring area.
 */
export function ringsInner(count: number): number {
  return count > 0 ? ringPair(count - 1, count).seven.inner : RING_OUTER;
}

/** The stroke-dasharray of a ring with pathLength 100. It clamps pct to 0..100. */
export function ringDash(pct: number): string {
  return Math.max(0, Math.min(100, pct)) + ' 100';
}
