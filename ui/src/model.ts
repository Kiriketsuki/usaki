// Turns the backend data into what the bubble draws. It holds no DOM code.

import { type CentreText, centreText, pickAuto } from './centre';
import { DEFAULT_THRESHOLDS, type Thresholds, tone } from './colour';
import { type EarState, earState } from './ears';
import { TEXT_MARGIN, type TextFit, fitText } from './fit';
import { ringsInner } from './geometry';
import {
  type AccountView,
  FIVE_HOUR_MINUTES,
  SEVEN_DAY_MINUTES,
  type UsageUpdate,
  type WindowNow,
  fiveHour,
  sevenDay,
  windowAt
} from './usage';

export interface RingView {
  pct: number;
  color: string;
}

/** The two rings of one account. A stale account dims both. */
export interface RingPairView {
  five: RingView;
  seven: RingView;
  dim: boolean;
}

export interface BubbleView {
  pairs: RingPairView[];
  ears: EarState;
  /** True when no account is live. The whole bubble then dims. */
  dim: boolean;
  text: CentreText;
  fit: TextFit;
  label: string;
}

function ringView(pct: number, thresholds: Thresholds): RingView {
  return { pct, color: tone(pct, thresholds) };
}

function spoken(w: WindowNow | null): string {
  return w ? Math.round(w.pct) + '%' : 'no data';
}

// The label carries the full account name, so a screen reader hears it even when the
// name fit cuts it on screen.
function ariaLabel(accounts: readonly AccountView[], text: CentreText, now: number): string {
  const shown = pickAuto(accounts, now);
  if (!shown || text.centre === '--') return 'AI usage. No fresh data.';
  return (
    'AI usage. ' + shown.id + ': ' +
    spoken(windowAt(shown, FIVE_HOUR_MINUTES, now)) + ' used in the 5 hour window, ' +
    spoken(windowAt(shown, SEVEN_DAY_MINUTES, now)) + ' used in the 7 day window. ' +
    'The 5 hour window resets in ' + text.centre + '.'
  );
}

/**
 * Returns the bubble at `now`. Each account has a pair of rings: the 5h window outside and
 * the 7d window inside. The pairs keep the order of the accounts, outermost first.
 */
export function bubbleView(
  update: UsageUpdate | null,
  now: number,
  thresholds: Thresholds = DEFAULT_THRESHOLDS
): BubbleView {
  const accounts = update?.accounts ?? [];
  const pairs = accounts.map((a) => ({
    five: ringView(fiveHour(a, now).pct, thresholds),
    seven: ringView(sevenDay(a, now).pct, thresholds),
    dim: a.stale
  }));
  // The ears follow the 5h rings of the live accounts, as in the approved fold rule.
  const live = pairs.filter((p) => !p.dim).map((p) => p.five.pct);
  const text = centreText(accounts, now);
  return {
    pairs,
    ears: earState(live, thresholds.crit),
    dim: live.length === 0,
    text,
    fit: fitText(text, ringsInner(pairs.length) - TEXT_MARGIN),
    label: ariaLabel(accounts, text, now)
  };
}
