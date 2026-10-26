// Draws a CardView into the card markup of index.html. The rows are a port of
// docs/design/mockup/Card.dc.html.

import { CARD_ROW, type AccountRow, type CardView, type WindowRow } from './card';
import { MOON_DISC } from './moon';

const SVG_NS = 'http://www.w3.org/2000/svg';
/** The opacity of the rows of a stale account, from the mockup. */
const STALE_ROWS = 0.5;

function svg(tag: string, attrs: Record<string, string | number>): SVGElement {
  const el = document.createElementNS(SVG_NS, tag);
  Object.entries(attrs).forEach(([k, v]) => el.setAttribute(k, String(v)));
  return el;
}

function html(tag: string, className: string, text = ''): HTMLElement {
  const el = document.createElement(tag);
  el.className = className;
  el.textContent = text;
  return el;
}

function part<T extends Element>(root: ParentNode, selector: string): T {
  const el = root.querySelector<T>(selector);
  if (!el) throw new Error('The card markup has no ' + selector + ' element.');
  return el;
}

// The hex dot of the mockup, a flat-top hexagon in a 12 by 11 box.
function dotNode(color: string): SVGElement {
  const el = svg('svg', { width: 12, height: 11, viewBox: '0 0 12 11', 'aria-hidden': 'true' });
  el.append(svg('path', { d: 'M3 0.5 L9 0.5 L11.5 5.5 L9 10.5 L3 10.5 L0.5 5.5 Z', fill: color }));
  return el;
}

// The stale mark. A diamond polygon stands in for the text glyph of the mockup.
function staleNode(text: string): HTMLElement {
  const el = html('span', 'stale');
  const mark = svg('svg', { width: 8, height: 8, viewBox: '0 0 8 8', 'aria-hidden': 'true' });
  mark.append(svg('polygon', { points: '4,0 8,4 4,8 0,4', fill: 'currentColor' }));
  el.append(mark, document.createTextNode(text));
  return el;
}

function moonNode(w: WindowRow): HTMLElement {
  const box = html('span', 'moon');
  if (!w.moon) return box;
  const el = svg('svg', { width: 12, height: 12, viewBox: '0 0 12 12', role: 'img', 'aria-label': w.moon.label });
  el.append(svg('path', { d: MOON_DISC, fill: '#252836', stroke: '#363a4f', 'stroke-width': 1 }));
  if (w.moon.lit) el.append(svg('path', { d: w.moon.lit, fill: '#e0e2ea' }));
  box.append(el);
  return box;
}

// The reset mark: a hex arc with a triangle head, both straight-edged, in place of the
// text glyph of the mockup.
function resetMark(): SVGElement {
  const el = svg('svg', { width: 9, height: 9, viewBox: '0 0 10 10', 'aria-hidden': 'true' });
  el.append(
    svg('polyline', { points: '8.5,3 7,0.8 3,0.8 1,4.5 3,8.8 7,8.8 8.6,6.3', fill: 'none', stroke: 'currentColor', 'stroke-width': 1.2 }),
    svg('polygon', { points: '9.8,0.6 9.6,4.4 6.2,3', fill: 'currentColor' })
  );
  return el;
}

function windowNode(w: WindowRow): HTMLElement {
  const row = html('div', 'win');
  const track = html('div', 'bar');
  const fill = html('div', 'fill');
  fill.style.width = w.bar + '%';
  fill.style.background = w.color;
  track.append(fill);
  const reset = html('span', 'reset' + (w.left === 'ready' ? ' ready' : ''));
  if (w.at) reset.append(resetMark());
  reset.append(document.createTextNode(' ' + w.left + ' '), html('span', 'at', w.at));
  row.append(html('span', 'label', w.label), track, html('span', 'pct', w.pct), moonNode(w), reset);
  return row;
}

function accountNode(a: AccountRow): HTMLElement {
  const block = html('div', 'acct');
  block.style.height = CARD_ROW + 'px';
  const title = html('div', 'title');
  title.append(dotNode(a.dot), html('span', 'name', a.id));
  if (a.tag) title.append(html('span', 'tag', a.tag));
  title.append(html('span', 'grow'));
  if (a.stale) title.append(staleNode(a.stale));
  const wins = html('div', 'wins');
  wins.style.opacity = a.stale ? String(STALE_ROWS) : '1';
  wins.append(...a.windows.map(windowNode));
  block.append(title, wins);
  return block;
}

function emptyNode(): HTMLElement {
  const block = html('div', 'acct empty', 'No account data yet.');
  block.style.height = CARD_ROW + 'px';
  return block;
}

export function renderCard(root: HTMLElement, view: CardView): void {
  root.style.width = view.width + 'px';
  root.style.height = view.height + 'px';
  part<HTMLElement>(root, '.updated').textContent = view.updated;
  part<HTMLElement>(root, '.rows').replaceChildren(...(view.rows.length ? view.rows.map(accountNode) : [emptyNode()]));
}
