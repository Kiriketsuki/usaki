// Turns the backend data into what the bubble draws. It holds no DOM code.

import { type CellText, NO_DATA, READY, type WindowText, cellText, pickAuto } from './centre';
import { DEFAULT_THRESHOLDS, type Thresholds, tone } from './colour';
import { type EarState, earState } from './ears';
import { type CellFit, fitCell } from './fit';
import { type Layout, layout } from './layout';
import { type AccountView, type UsageUpdate, fiveHour, sevenDay } from './usage';

export interface RingView {
  pct: number;
  color: string;
}

/** One hex cell. A cell with no rings is the no data cell. */
export interface CellView {
  x: number;
  y: number;
  rings: { five: RingView; seven: RingView } | null;
  text: CellText;
  fit: CellFit;
  /** True for a stale account. The cell then dims. */
  dim: boolean;
  /** True for the auto account, the one whose 5h window resets next. */
  highlight: boolean;
}

export interface BubbleView {
  cells: CellView[];
  layout: Layout;
  ears: EarState;
  /** True when no account is live. The whole bubble then dims. */
  dim: boolean;
  label: string;
}

function ringView(pct: number, thresholds: Thresholds): RingView {
  return { pct, color: tone(pct, thresholds) };
}

function spoken(w: WindowText): string {
  if (w.pct === '--') return 'no data';
  if (w.countdown === READY) return w.pct + ' used, ready';
  return w.pct + ' used, resets in ' + w.countdown;
}

// The label carries the full account name, so a screen reader hears it even when the
// fit cuts it on screen.
function ariaLabel(shown: AccountView | null, now: number): string {
  if (!shown) return 'AI usage. No fresh data.';
  const t = cellText(shown, now);
  return (
    'AI usage. Next reset: ' + t.name + '. 5 hour window: ' + spoken(t.five) +
    '. 7 day window: ' + spoken(t.seven) + '.'
  );
}

function noDataCell(at: { x: number; y: number }): CellView {
  return { ...at, rings: null, text: NO_DATA, fit: fitCell(NO_DATA), dim: false, highlight: false };
}

/**
 * Returns the bubble at `now`: one hex cell per account, in account order, tiled into one
 * cluster. Each cell has a 7d ring outside and a 5h ring inside it.
 */
export function bubbleView(
  update: UsageUpdate | null,
  now: number,
  thresholds: Thresholds = DEFAULT_THRESHOLDS
): BubbleView {
  const accounts = update?.accounts ?? [];
  const lay = layout(accounts.length);
  const shown = pickAuto(accounts, now);
  const cells =
    accounts.length === 0
      ? [noDataCell(lay.cells[0])]
      : accounts.map((a, i) => {
          const text = cellText(a, now);
          return {
            ...lay.cells[i],
            rings: {
              five: ringView(fiveHour(a, now).pct, thresholds),
              seven: ringView(sevenDay(a, now).pct, thresholds)
            },
            text,
            fit: fitCell(text),
            dim: a.stale,
            highlight: a === shown
          };
        });
  // The right ear follows the live 5h windows and the left ear the live 7d windows.
  const live = accounts
    .filter((a) => !a.stale)
    .map((a) => ({ five: fiveHour(a, now).pct, seven: sevenDay(a, now).pct }));
  return {
    cells,
    layout: lay,
    ears: earState(live, thresholds.crit),
    dim: live.length === 0,
    label: ariaLabel(shown, now)
  };
}
