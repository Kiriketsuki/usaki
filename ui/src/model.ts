// Turns the backend data into what the bubble draws. It holds no DOM code.

import { type CentreText, centreText } from './centre';
import { DEFAULT_THRESHOLDS, type Thresholds, tone } from './colour';
import { type EarState, earState } from './ears';
import { type UsageUpdate, fiveHour } from './usage';

export interface RingView {
  pct: number;
  color: string;
  dim: boolean;
}

export interface BubbleView {
  rings: RingView[];
  ears: EarState;
  /** True when no account is live. The whole bubble then dims. */
  dim: boolean;
  text: CentreText;
  label: string;
}

function ariaLabel(text: CentreText): string {
  if (text.centre === '--') return 'AI usage. No fresh data.';
  return 'AI usage. ' + text.sub + ' used in the 5 hour window, resets in ' + text.centre + '.';
}

/** Returns the bubble at `now`. The rings show the 5h window, one ring per account. */
export function bubbleView(
  update: UsageUpdate | null,
  now: number,
  thresholds: Thresholds = DEFAULT_THRESHOLDS
): BubbleView {
  const accounts = update?.accounts ?? [];
  const rings = accounts.map((a) => {
    const pct = fiveHour(a, now).pct;
    return { pct, color: tone(pct, thresholds), dim: a.stale };
  });
  const live = rings.filter((r) => !r.dim).map((r) => r.pct);
  const text = centreText(accounts, now);
  return {
    rings,
    ears: earState(live, thresholds.crit),
    dim: live.length === 0,
    text,
    label: ariaLabel(text)
  };
}
