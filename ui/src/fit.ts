// Fits the three centre lines inside the innermost ring. The page draws the lines in Fira
// Code, a monospace font, so the width of a line follows from its length. This keeps the
// fit a pure function, with no DOM measure.

import { APOTHEM } from './geometry';
import { type CentreText, accountLine } from './centre';

/** The advance of one Fira Code glyph, as a fraction of the font size. */
const CHAR_EM = 0.6;
/** The flex gap between two lines, in px. It matches .text in style.css. */
const LINE_GAP = 1;
/** The line-height of each line. They match .top, .centre and .sub in style.css. */
const LINE_HEIGHT = { top: 1.2, centre: 1.1, sub: 1.2 } as const;
/** The clear space between the text and the inner edge of the innermost ring, in px. */
export const TEXT_MARGIN = 1;
/** The widest countdown of the 5h window. The size step uses it, so the step stays put. */
const WIDEST_COUNTDOWN = '4h 59m';
/** The widest figures in normal use. A figure of 100 or more can still widen the line. */
const WIDEST_FIGURES = '99/99%';
/** The size step keeps room for at least this many name characters before the ellipsis. */
const MIN_NAME = 6;
const ELLIPSIS = '\u2026';

/** Font sizes in px. */
export interface TextSizes {
  top: number;
  centre: number;
  sub: number;
}

// The size steps, largest first. Step 0 is the mockup.
const STEPS: readonly TextSizes[] = [
  { top: 9, centre: 17, sub: 10 },
  { top: 8, centre: 15, sub: 9 }
];
/** The account line drops at most this many px below its step to show the full name. */
const SUB_DROP = 1;

/**
 * The fitted centre: the sizes, the account line with the name cut if needed, and the move
 * of the text block in px. A negative offset moves the block up.
 */
export interface TextFit {
  sizes: TextSizes;
  sub: string;
  offset: number;
}

/** One centre line, as the fit sees it. */
export interface Line {
  text: string;
  size: number;
  lineHeight: number;
}

/**
 * Returns the hex norm of each line: the apothem of the smallest centred hexagon that
 * holds the em box of the line. The lines stack in a flex column, centred and then moved
 * down by `offset` px. A negative offset moves the lines up.
 */
export function lineNorms(lines: readonly Line[], offset = 0): number[] {
  const boxes = lines.map((l) => l.size * l.lineHeight);
  const total = boxes.reduce((a, b) => a + b, 0) + LINE_GAP * (lines.length - 1);
  let y = offset - total / 2;
  return lines.map((l, i) => {
    const mid = y + boxes[i] / 2;
    y += boxes[i] + LINE_GAP;
    const dy = Math.abs(mid) + l.size / 2;
    const dx = (l.text.length * CHAR_EM * l.size) / 2;
    // A flat-top hex has its top and bottom edges at the apothem, and its four slanted
    // edges at x * cos 30 + y * sin 30 = apothem.
    return Math.max(dy, dx * APOTHEM + dy / 2);
  });
}

/** The largest move of the text block, in px, and the step of the search. */
const MAX_OFFSET = 10;
const OFFSET_STEP = 0.5;

/**
 * Returns the move of the text block that fits it inside a hexagon of apothem `limit`, or
 * null when no move fits. The account line is the widest line and sits at the bottom,
 * near the slanted edges. A move up takes it toward the wide middle of the hex. The
 * search starts at 0 and goes out, so the block stays as near the centre as it can.
 */
export function place(lines: readonly Line[], limit: number): number | null {
  for (let o = 0; o <= MAX_OFFSET; o += OFFSET_STEP) {
    for (const offset of o === 0 ? [0] : [-o, o]) {
      if (lineNorms(lines, offset).every((n) => n <= limit)) return offset;
    }
  }
  return null;
}

/** The three centre lines with their sizes and the line-heights of style.css. */
export function linesOf(top: string, centre: string, sub: string, sizes: TextSizes): Line[] {
  return [
    { text: top, size: sizes.top, lineHeight: LINE_HEIGHT.top },
    { text: centre, size: sizes.centre, lineHeight: LINE_HEIGHT.centre },
    { text: sub, size: sizes.sub, lineHeight: LINE_HEIGHT.sub }
  ];
}

// The widest of the real countdown and WIDEST_COUNTDOWN. The size step then does not
// change as the countdown runs down.
function widest(centre: string): string {
  return centre.length > WIDEST_COUNTDOWN.length ? centre : WIDEST_COUNTDOWN;
}

function cut(name: string, keep: number): string {
  return keep >= name.length ? name : name.slice(0, keep) + ELLIPSIS;
}

/**
 * Returns the size step for a hexagon of apothem `limit`. It is the largest step that fits
 * the mode line, the widest countdown and the widest account line with a cut name. The
 * step depends on the ring count only, so the countdown keeps its size when the shown
 * account changes.
 */
export function sizeStep(top: string, limit: number): TextSizes {
  const sub = accountLine(cut('x'.repeat(MIN_NAME + 1), MIN_NAME), WIDEST_FIGURES);
  const fits = (s: TextSizes) =>
    place(linesOf(top, WIDEST_COUNTDOWN, sub, { ...s, sub: s.sub - SUB_DROP }), limit) != null;
  return STEPS.find(fits) ?? STEPS[STEPS.length - 1];
}

/**
 * Fits the centre text inside a hexagon of apothem `limit`. The rule:
 *
 * 1. The text takes the size step from sizeStep.
 * 2. The account line keeps the size of that step, or drops SUB_DROP px, to show the
 *    full name.
 * 3. If the full name still does not fit, the name loses characters from the end and ends
 *    in an ellipsis. The figures always stay whole.
 *
 * Each try moves the block up or down by the smallest amount that fits it.
 */
export function fitText(text: CentreText, limit: number): TextFit {
  const head = widest(text.centre);
  const step = sizeStep(text.top, limit);
  const tries: { sizes: TextSizes; sub: string }[] = [];
  for (let drop = 0; drop <= SUB_DROP; drop += 1) {
    tries.push({ sizes: { ...step, sub: step.sub - drop }, sub: accountLine(text.name, text.figures) });
  }
  const small = { ...step, sub: step.sub - SUB_DROP };
  for (let keep = text.name.length - 1; keep >= 1; keep -= 1) {
    tries.push({ sizes: small, sub: accountLine(cut(text.name, keep), text.figures) });
  }
  for (const t of tries) {
    const offset = place(linesOf(text.top, head, t.sub, t.sizes), limit);
    if (offset != null) return { ...t, offset };
  }
  // No try fits. The ring specs keep this from happening for up to 5 accounts.
  const last = tries[tries.length - 1];
  return { ...last, offset: 0 };
}
