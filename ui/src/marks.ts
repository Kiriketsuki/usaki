// The provider name marks. Each cell shows a small filled polygon before the account name.
// The shape names the provider without colour: a starburst for Claude, a chevron for Codex.
// Chrysaki asks for straight edges, so both marks are polygons with no curve.

import type { ProviderName } from './usage';

/**
 * The accent of each provider: the cell mark and the card dot. Claude uses its own brand
 * terracotta and Codex a teal. The rings keep the usage tones, and the mark shape carries
 * the provider, so an accent close to a usage tone stays readable.
 */
export const PROVIDER_COLOUR: Record<ProviderName, string> = {
  claude: '#D97757',
  codex: '#20969c'
};

/** The name of each provider, for the bubble label. */
export const PROVIDER_LABEL: Record<ProviderName, string> = {
  claude: 'Claude',
  codex: 'Codex'
};

type Point = readonly [number, number];

/** The rays of the starburst. */
const RAYS = 8;
/** The inner radius of the starburst, as a fraction of the box. The outer radius is 0.5. */
const RAY_INNER = 0.22;

// The starburst alternates an outer and an inner point, from 12 o'clock.
const STARBURST: readonly Point[] = Array.from({ length: RAYS * 2 }, (_, i) => {
  const r = i % 2 === 0 ? 0.5 : RAY_INNER;
  const a = (Math.PI * i) / RAYS - Math.PI / 2;
  return [0.5 + r * Math.cos(a), 0.5 + r * Math.sin(a)] as const;
});

// A thick `>` with a flat back and a sharp point on the right.
const CHEVRON: readonly Point[] = [
  [0.1, 0.04],
  [0.48, 0.04],
  [0.92, 0.5],
  [0.48, 0.96],
  [0.1, 0.96],
  [0.54, 0.5]
];

const SHAPES: Record<ProviderName, readonly Point[]> = { claude: STARBURST, codex: CHEVRON };

/** Returns the polygon points of the mark of `provider` in a `size` px square box. */
export function markPoints(provider: ProviderName, size: number): string {
  return SHAPES[provider].map(([x, y]) => +(x * size).toFixed(2) + ',' + +(y * size).toFixed(2)).join(' ');
}
