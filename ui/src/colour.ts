// Ring colours and thresholds, ported from docs/design/mockup/Main.dc.html.

export const EMERALD = '#1a8a6a';
export const BLONDE = '#FBB13C';
export const RUBY = '#e06a78';

/** The % used at which a ring turns blonde (warn) and ruby (crit). */
export interface Thresholds {
  warn: number;
  crit: number;
}

export const DEFAULT_THRESHOLDS: Thresholds = { warn: 50, crit: 75 };

/** Emerald under `warn`, blonde from `warn`, ruby from `crit`. */
export function tone(pct: number, t: Thresholds = DEFAULT_THRESHOLDS): string {
  if (pct >= t.crit) return RUBY;
  if (pct >= t.warn) return BLONDE;
  return EMERALD;
}
