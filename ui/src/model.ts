// Turns the backend data into what the bubble draws. It holds no DOM code.

import { AUTO, type CellText, type HeadMode, NO_DATA, type Pin, READY, type WindowText, cellText, pickHead } from './centre';
import { DEFAULT_THRESHOLDS, type Thresholds, tone } from './colour';
import { type EarState, earState } from './ears';
import { type CellFit, fitCell } from './fit';
import { type Cell, type Layout, layout } from './layout';
import { PROVIDER_LABEL } from './marks';
import {
  type AccountView,
  FIVE_HOUR_MINUTES,
  SEVEN_DAY_MINUTES,
  type UsageUpdate,
  fiveHour,
  hasWindow,
  noData,
  sevenDay,
  weekOnly
} from './usage';

export interface RingView {
  pct: number;
  color: string;
}

/** One hex cell. A cell with no rings is the no data cell. */
export interface CellView {
  x: number;
  y: number;
  /** The radius in px. The head is bigger than a body cell. */
  r: number;
  /** The id of the account in the cell, or null for the no data cell. */
  id: string | null;
  /**
   * A ring is null when the account has no such window, such as the 5h ring of a 7d-only
   * account. The cell then draws a dotted empty track.
   */
  rings: { five: RingView | null; seven: RingView | null } | null;
  text: CellText;
  fit: CellFit;
  /** True for a stale account. The cell then dims. */
  dim: boolean;
  /** True for the head when it is the auto account or a pinned account. */
  highlight: boolean;
}

export interface BubbleView {
  cells: CellView[];
  layout: Layout;
  ears: EarState;
  /** How the head was chosen. A pinned head gets its own outline tone and a stud. */
  mode: HeadMode;
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

/** The control hint at the end of the bubble label. */
export const CONTROLS_HINT =
  ' Click to open the card. Right-click pins the next account. Middle-click returns to auto.';

// The label carries the full account name, so a screen reader hears it even when the
// fit cuts it on screen.
function ariaLabel(head: AccountView | null, mode: HeadMode, now: number): string {
  if (!head || mode === 'none') return 'AI usage. No fresh data.' + CONTROLS_HINT;
  const t = cellText(head, now);
  const who = (mode === 'pinned' ? 'Pinned: ' : 'Next reset: ') + PROVIDER_LABEL[head.provider] + ' ' + t.name;
  if (noData(head)) return 'AI usage. ' + who + '. No data.' + CONTROLS_HINT;
  const five = weekOnly(head) ? 'none' : spoken(t.five);
  return 'AI usage. ' + who + '. 5 hour window: ' + five + '. 7 day window: ' + spoken(t.seven) + '.' + CONTROLS_HINT;
}

function noDataCell(at: Cell): CellView {
  return { ...at, id: null, rings: null, text: NO_DATA, fit: fitCell(NO_DATA), dim: false, highlight: false };
}

/**
 * Puts the accounts in cell order. The head is cell 0. The others follow in account order
 * as the body.
 */
export function headFirst(accounts: readonly AccountView[], head: AccountView | null): AccountView[] {
  return head ? [head, ...accounts.filter((a) => a !== head)] : [...accounts];
}

/**
 * Returns the bubble at `now`: one hex cell per account, laid out as a rabbit. The head is
 * the pinned account, or else the auto account, and it carries the ears. Each cell has a 7d
 * ring outside and a 5h ring inside it.
 */
export function bubbleView(
  update: UsageUpdate | null,
  now: number,
  pin: Pin = AUTO,
  thresholds: Thresholds = DEFAULT_THRESHOLDS
): BubbleView {
  const accounts = update?.accounts ?? [];
  const lay = layout(accounts.length);
  const { account: shown, mode } = pickHead(accounts, pin, now);
  const cells =
    accounts.length === 0
      ? [noDataCell(lay.cells[0])]
      : headFirst(accounts, shown).map((a, i) => {
          const text = cellText(a, now);
          return {
            ...lay.cells[i],
            id: a.id,
            rings: {
              five: hasWindow(a, FIVE_HOUR_MINUTES) ? ringView(fiveHour(a, now).pct, thresholds) : null,
              seven: hasWindow(a, SEVEN_DAY_MINUTES) ? ringView(sevenDay(a, now).pct, thresholds) : null
            },
            text,
            fit: fitCell(text),
            dim: a.stale,
            highlight: a === shown && mode !== 'none'
          };
        });
  // The right ear follows the live 5h windows and the left ear the live 7d windows. A 7d-only
  // account has no 5h window, so it counts for the left ear only.
  const live = accounts
    .filter((a) => !a.stale)
    .map((a) => ({ five: hasWindow(a, FIVE_HOUR_MINUTES) ? fiveHour(a, now).pct : null, seven: sevenDay(a, now).pct }));
  return {
    cells,
    layout: lay,
    ears: earState(live, thresholds.crit),
    mode,
    dim: live.length === 0,
    label: ariaLabel(shown, mode, now)
  };
}
