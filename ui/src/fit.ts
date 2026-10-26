// Lays out the text of a cell and fits it inside the 5h ring. The page draws the text in
// Fira Code, a monospace font, so the width of a text follows from its length. This keeps
// the fit a pure function, with no DOM measure.
//
// The cell text has three rows on one common centre. The top row holds the 5h percent on
// the left and the 7d percent on the right. The middle row holds the provider mark and the
// name, where the hex is widest. The mark and the name centre on the cell as one group. The
// bottom row holds the 5h countdown on the left and the 7d countdown on the right. The two
// columns sit at fixed x, so each percent lines up with its countdown.

import { type CellText, NO_DATA_TEXT } from './centre';
import { APOTHEM, FIVE_RING } from './geometry';

/** The advance of one Fira Code glyph, as a fraction of the font size. */
const CHAR_EM = 0.6;
/** The clear space between the text and the inner edge of the 5h ring, in px. */
export const TEXT_MARGIN = 1;
/** The text must stay inside a hexagon of this apothem. */
export const TEXT_LIMIT = FIVE_RING.inner - TEXT_MARGIN;
/** The x of the middle of each column, left of and right of the cell centre, in px. */
export const COL_X = 14.5;
/** The gap between two rows, in px. */
const ROW_GAP = 1.5;
const ELLIPSIS = '…';
/** The side of the square box of the provider mark, in px. */
export const MARK_SIZE = 8;
/** The gap between the mark and the name, in px. */
export const MARK_GAP = 1;

export type Role = 'pct' | 'name' | 'countdown';

/** The font size of each row in px. style.css sets the same sizes. */
export const ROW_SIZE: Record<Role, number> = { pct: 10.5, name: 9.5, countdown: 8 };
const ROWS: readonly Role[] = ['pct', 'name', 'countdown'];
/**
 * The name font sizes that the fit tries, largest first. The mark takes room on the name
 * row, so a long name steps down by up to 1 px before it loses characters. A 12 character
 * name then fits whole.
 */
export const NAME_SIZES: readonly number[] = [ROW_SIZE.name, ROW_SIZE.name - 0.5, ROW_SIZE.name - 1];

/** One text item, centred on (x, y) px from the cell centre, in a font of `size` px. */
export interface TextItem {
  text: string;
  role: Role;
  x: number;
  y: number;
  size: number;
}

// The y of the middle of each row. The rows stack on the cell centre, one em box each.
function rowY(): Record<Role, number> {
  const total = ROWS.reduce((sum, r) => sum + ROW_SIZE[r], 0) + ROW_GAP * (ROWS.length - 1);
  let y = -total / 2;
  const out = {} as Record<Role, number>;
  for (const r of ROWS) {
    out[r] = y + ROW_SIZE[r] / 2;
    y += ROW_SIZE[r] + ROW_GAP;
  }
  return out;
}

// The space that the mark takes before the name, in px.
function markLead(text: CellText): number {
  return text.provider ? MARK_SIZE + MARK_GAP : 0;
}

/**
 * The items of a cell, with the name as given. With a mark, the name moves right by half
 * the mark lead, so the mark and the name centre on the cell together. The rows keep their
 * places when the name font steps down.
 */
export function cellItems(text: CellText, name = text.name, nameSize = ROW_SIZE.name): TextItem[] {
  const y = rowY();
  const at = (t: string, role: Role, x: number): TextItem => ({ text: t, role, x, y: y[role], size: ROW_SIZE[role] });
  const nameItem: TextItem = { ...at(name, 'name', markLead(text) / 2), size: nameSize };
  // An account with no window shows `no data` on the countdown row, with no percent.
  if (text.noData) return [nameItem, at(NO_DATA_TEXT, 'countdown', 0)];
  return [
    at(text.five.pct, 'pct', -COL_X),
    at(text.seven.pct, 'pct', COL_X),
    nameItem,
    at(text.five.countdown, 'countdown', -COL_X),
    at(text.seven.countdown, 'countdown', COL_X)
  ];
}

/**
 * Returns the hex norm of an item: the apothem of the smallest centred hexagon that holds
 * its em box. A flat-top hex has its top and bottom edges at the apothem, and its four
 * slanted edges at x * cos 30 + y * sin 30 = apothem.
 */
export function itemNorm(item: TextItem): number {
  const dx = Math.abs(item.x) + itemHalfWidth(item);
  const dy = Math.abs(item.y) + item.size / 2;
  return Math.max(dy, dx * APOTHEM + dy / 2);
}

/** Returns the half width of an item, in px. */
export function itemHalfWidth(item: TextItem): number {
  return (item.text.length * CHAR_EM * item.size) / 2;
}

function fits(items: readonly TextItem[]): boolean {
  return items.every((i) => itemNorm(i) <= TEXT_LIMIT);
}

function cut(name: string, keep: number): string {
  return keep >= name.length ? name : name.slice(0, keep) + ELLIPSIS;
}

/** The centre of the provider mark, in px from the cell centre. */
export interface MarkPlace {
  x: number;
  y: number;
}

/** The fitted cell text: the name, cut if needed, the placed items and the mark. */
export interface CellFit {
  name: string;
  items: TextItem[];
  /** The mark centre, or null for a cell with no provider. */
  mark: MarkPlace | null;
}

// The mark sits at the left end of the name group.
function markPlace(text: CellText, items: readonly TextItem[]): MarkPlace | null {
  const name = items.find((i) => i.role === 'name');
  if (!text.provider || !name) return null;
  return { x: name.x - itemHalfWidth(name) - MARK_GAP - MARK_SIZE / 2, y: name.y };
}

/**
 * Fits the cell text inside the 5h ring. A 12 character name fits whole after the mark, at
 * a name font up to 1 px smaller. A longer name loses characters from the end and ends in an
 * ellipsis. The fit keeps the most characters first and the biggest font second. The
 * percents and the countdowns always stay whole, and the rows never move off the common
 * centre. The name item with its offset spans the mark too, so the fit of the name also fits
 * the mark.
 */
export function fitCell(text: CellText): CellFit {
  for (let keep = text.name.length; keep >= 1; keep -= 1) {
    const name = cut(text.name, keep);
    for (const size of NAME_SIZES) {
      const items = cellItems(text, name, size);
      if (fits(items)) return { name, items, mark: markPlace(text, items) };
    }
  }
  // No cut fits. Only a percent of 1000 or more can cause this.
  const name = cut(text.name, 1);
  const items = cellItems(text, name);
  return { name, items, mark: markPlace(text, items) };
}
