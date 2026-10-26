// Draws a BubbleView into the markup of index.html. The ring and outline attributes are a
// port of docs/design/mockup/Bubble.dc.html, drawn once per cell.

import { READY } from './centre';
import { ears } from './ears';
import { MARK_SIZE } from './fit';
import {
  APOTHEM,
  CELL_R,
  type RingGeometry,
  TRACK_DOT_SPACING,
  cellRings,
  hexCorners,
  hexPath,
  ringDash,
  ringNotch,
  trackDots
} from './geometry';
import type { Decor } from './layout';
import { PROVIDER_COLOUR, markPoints } from './marks';
import { PLATE_EDGE_W, platePath, plateHexes } from './plate';
import type { BubbleView, CellView, RingView } from './model';

const SVG_NS = 'http://www.w3.org/2000/svg';
const TRACK = '#252836';
/** The dots of an empty track, for a window that the account does not have. */
const NO_WINDOW_DOTS = '#363a4f';
/** The radius of one hex dot of an empty track, as a fraction of the ring stroke. */
const TRACK_DOT_R = 0.8;
/** The notch takes the glass tone, so it reads as a cut in the ring. */
const NOTCH = '#161821';
const EDGE = 'rgba(255,255,255,0.12)';
/** The light line along the outside of the plate: the Chrysaki text tone at a low opacity. */
const PLATE_EDGE = '#e0e2ea';
const PLATE_EDGE_OPACITY = 0.2;
const PLATE_MASK_ID = 'plate-mask';
const HIGHLIGHT = '#e0e2ea';
const HIGHLIGHT_SW = 1.5;
/** A pinned head swaps the bright outline for the amethyst of the inner ear. */
const PINNED = '#7a4fb8';
/** The pin stud between the ears: a Blonde diamond, like the eye stud of the rabbit-head mark. */
const STUD = '#FBB13C';
const STUD_HALF_W = 3.5;
const STUD_HALF_H = 4.5;
/** The gap between the stud tip and the top edge of the head, in px. */
const STUD_LIFT = 1.5;
/** The opacity of the rings and of the text of a stale cell. The text stays brighter, so it still reads. */
const STALE_RINGS = 0.4;
const STALE_TEXT = 0.6;

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
  if (!el) throw new Error('The bubble markup has no ' + selector + ' element.');
  return el;
}

// A small decor hex of the ears or the foot pads. Decor has no ring and no text.
function decorNode(d: Decor): SVGElement {
  return svg('polygon', { points: hexCorners(d.r, d.x, d.y).map((p) => p.join(',')).join(' '), fill: d.fill });
}

// The decor hexes of the current fold state: both ears, then the foot pads.
function decorOf(view: BubbleView): Decor[] {
  const top = view.layout.cells[0];
  return [...ears(view.ears, top.x, view.layout.earBase).flat(), ...view.layout.pads];
}

// The light line along the outside of the plate. Each padded hex gets a stroke, and a mask
// hides every part of a stroke that falls on the plate. Only the outer half of the outline of
// the union stays, PLATE_EDGE_W wide, with no line where two padded hexes overlap.
function plateEdgeNodes(d: string, width: number, height: number): SVGElement[] {
  const mask = svg('mask', { id: PLATE_MASK_ID, maskUnits: 'userSpaceOnUse', x: 0, y: 0, width, height });
  mask.append(svg('rect', { width, height, fill: '#fff' }), svg('path', { d, fill: '#000' }));
  const defs = svg('defs', {});
  defs.append(mask);
  const edge = svg('path', {
    d,
    fill: 'none',
    stroke: PLATE_EDGE,
    'stroke-opacity': PLATE_EDGE_OPACITY,
    'stroke-width': 2 * PLATE_EDGE_W,
    mask: 'url(#' + PLATE_MASK_ID + ')'
  });
  return [defs, edge];
}

// A ring is a track path and a progress path on top of it, as in the mockup.
function ringNodes(c: CellView, g: RingGeometry, ring: RingView): SVGElement[] {
  const d = hexPath(g.r, c.x, c.y);
  return [
    svg('path', { d, fill: 'none', stroke: TRACK, 'stroke-width': g.sw }),
    svg('path', {
      d,
      fill: 'none',
      stroke: ring.color,
      'stroke-width': g.sw,
      pathLength: 100,
      'stroke-dasharray': ringDash(ring.pct),
      'stroke-linecap': 'butt'
    }),
    svg('polygon', { points: ringNotch(g, c.r, c.x, c.y), fill: NOTCH })
  ];
}

// A window that the account does not have draws an empty track of small flat-top hex dots,
// with no arc and no notch. The dots read as empty, not as a low percent. The spacing scales
// with the cell, so the head track is the body track scaled.
function emptyRingNode(c: CellView, g: RingGeometry): SVGElement {
  const el = svg('g', { fill: NO_WINDOW_DOTS });
  const dotR = g.sw * TRACK_DOT_R;
  el.append(
    ...trackDots(g.r, c.x, c.y, (TRACK_DOT_SPACING * c.r) / CELL_R).map(([x, y]) =>
      svg('polygon', { points: hexCorners(dotR, x, y).map((p) => p.join(',')).join(' ') })
    )
  );
  return el;
}

function cellNode(c: CellView): SVGElement {
  const g = svg('g', { opacity: c.dim ? STALE_RINGS : 1 });
  g.append(svg('path', { d: hexPath(c.r, c.x, c.y), fill: 'none', stroke: EDGE, 'stroke-width': 1 }));
  const rings = cellRings(c.r);
  if (c.rings) {
    const { five, seven } = c.rings;
    g.append(
      ...(seven ? ringNodes(c, rings.seven, seven) : [emptyRingNode(c, rings.seven)]),
      ...(five ? ringNodes(c, rings.five, five) : [emptyRingNode(c, rings.five)])
    );
  }
  return g;
}

// The provider mark before the name. The fit gives its centre on a body cell.
function markNode(c: CellView, k: number): SVGElement | null {
  const place = c.fit.mark;
  const provider = c.text.provider;
  if (!place || !provider) return null;
  const size = MARK_SIZE * k;
  const el = svg('svg', { class: 'mark', width: size, height: size, viewBox: '0 0 ' + size + ' ' + size, 'aria-hidden': 'true' });
  el.style.left = place.x * k + 'px';
  el.style.top = place.y * k + 'px';
  el.append(svg('polygon', { points: markPoints(provider, size), fill: PROVIDER_COLOUR[provider] }));
  return el;
}

// The highlight runs just inside the cell edge, so it does not cover the next cell.
function highlightNode(c: CellView, pinned: boolean): SVGElement {
  const r = (c.r * APOTHEM - HIGHLIGHT_SW / 2) / APOTHEM;
  return svg('path', {
    d: hexPath(r, c.x, c.y),
    fill: 'none',
    stroke: pinned ? PINNED : HIGHLIGHT,
    'stroke-opacity': pinned ? 1 : 0.7,
    'stroke-width': HIGHLIGHT_SW
  });
}

// The stud sits in the gap between the two ear bases, above the top edge of the head, so it
// covers no ring and no text.
function studNode(c: CellView): SVGElement {
  const tip = c.y - c.r * APOTHEM - STUD_LIFT;
  const mid = tip - STUD_HALF_H;
  const points = [
    [c.x, tip],
    [c.x + STUD_HALF_W, mid],
    [c.x, mid - STUD_HALF_H],
    [c.x - STUD_HALF_W, mid]
  ];
  return svg('polygon', { points: points.map((p) => p.join(',')).join(' '), fill: STUD });
}

// Each text item sits on its own point around the cell centre, so the rows share one
// centre and each percent lines up with its countdown.
function labelNode(c: CellView): HTMLElement {
  const el = html('div', 'cell-text');
  el.style.left = c.x + 'px';
  el.style.top = c.y + 'px';
  if (c.dim) el.style.opacity = String(STALE_TEXT);
  // The fit works on a body cell. A bigger cell scales the whole text block with it.
  const k = c.r / CELL_R;
  const mark = markNode(c, k);
  if (mark) el.append(mark);
  el.append(
    ...c.fit.items.map((item) => {
      const span = html('span', item.role + (item.text === READY ? ' ready' : ''), item.text);
      span.style.left = item.x * k + 'px';
      span.style.top = item.y * k + 'px';
      span.style.fontSize = item.size * k + 'px';
      return span;
    })
  );
  return el;
}

export function renderBubble(root: HTMLElement, view: BubbleView): void {
  const { width, height, cells } = view.layout;
  root.style.width = width + 'px';
  root.style.height = height + 'px';
  root.style.opacity = view.dim ? '0.55' : '1';
  root.setAttribute('aria-label', view.label);
  for (const sel of ['.plate', '.cells']) {
    const el = part<SVGElement>(root, sel);
    el.setAttribute('width', String(width));
    el.setAttribute('height', String(height));
    el.setAttribute('viewBox', '0 0 ' + width + ' ' + height);
  }
  // One glass fills the whole plate, so the cells, the ears and the feet read as one rabbit.
  const decor = decorOf(view);
  const plate = platePath(plateHexes([...cells, ...decor]));
  part<HTMLElement>(root, '.glass').style.clipPath = "path('" + plate + "')";
  part<SVGElement>(root, '.plate').replaceChildren(...plateEdgeNodes(plate, width, height));
  const pinned = view.mode === 'pinned';
  const head = view.cells[0];
  part<SVGElement>(root, '.cells').replaceChildren(
    ...decor.map(decorNode),
    ...view.cells.map(cellNode),
    ...view.cells.filter((c) => c.highlight).map((c) => highlightNode(c, pinned)),
    ...(pinned && head ? [studNode(head)] : [])
  );
  part<HTMLElement>(root, '.labels').replaceChildren(...view.cells.map(labelNode));
}
