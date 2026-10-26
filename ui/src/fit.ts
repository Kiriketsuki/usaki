// Fits the three lines of a cell inside its 7d ring. The page draws the lines in Fira Code,
// a monospace font, so the width of a line follows from its length. This keeps the fit a
// pure function, with no DOM measure.

import { type CellText } from './centre';
import { APOTHEM, SEVEN_RING } from './geometry';

/** The advance of one Fira Code glyph, as a fraction of the font size. */
const CHAR_EM = 0.6;
/** The flex gap between two lines, in px. It matches .cell-text in style.css. */
const LINE_GAP = 1;
/** The clear space between the text and the inner edge of the 7d ring, in px. */
export const TEXT_MARGIN = 1;
/** The text must stay inside a hexagon of this apothem. */
export const TEXT_LIMIT = SEVEN_RING.inner - TEXT_MARGIN;
const ELLIPSIS = '…';

/** Font size and line-height of each line. They match .countdown, .name and .figures in style.css. */
export const LINE_STYLE = {
  countdown: { size: 12, lineHeight: 1.1 },
  name: { size: 8, lineHeight: 1.2 },
  figures: { size: 8, lineHeight: 1.2 }
} as const;

/** One text line, as the fit sees it. */
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
const MAX_OFFSET = 6;
const OFFSET_STEP = 0.5;

/**
 * Returns the move of the text block that fits it inside a hexagon of apothem `limit`, or
 * null when no move fits. The search starts at 0 and goes out, so the block stays as near
 * the centre as it can.
 */
export function place(lines: readonly Line[], limit: number): number | null {
  for (let o = 0; o <= MAX_OFFSET; o += OFFSET_STEP) {
    for (const offset of o === 0 ? [0] : [-o, o]) {
      if (lineNorms(lines, offset).every((n) => n <= limit)) return offset;
    }
  }
  return null;
}

/** The three lines of a cell, with the sizes of LINE_STYLE. */
export function cellLines(text: CellText): Line[] {
  return [
    { text: text.countdown, ...LINE_STYLE.countdown },
    { text: text.name, ...LINE_STYLE.name },
    { text: text.figures, ...LINE_STYLE.figures }
  ];
}

/** The fitted cell text: the name, cut if needed, and the move of the block in px. */
export interface CellFit {
  name: string;
  offset: number;
}

function cut(name: string, keep: number): string {
  return keep >= name.length ? name : name.slice(0, keep) + ELLIPSIS;
}

/**
 * Fits the cell text inside the 7d ring. The name sits on the middle line, where the hex is
 * widest, so a 12 character name fits whole. A longer name loses characters from the end
 * and ends in an ellipsis. The countdown and the figures always stay whole.
 */
export function fitCell(text: CellText): CellFit {
  for (let keep = text.name.length; keep >= 1; keep -= 1) {
    const name = cut(text.name, keep);
    const offset = place(cellLines({ ...text, name }), TEXT_LIMIT);
    if (offset != null) return { name, offset };
  }
  // No cut fits. Only figures far over 100% can cause this. The block then stays centred.
  return { name: cut(text.name, 1), offset: 0 };
}
