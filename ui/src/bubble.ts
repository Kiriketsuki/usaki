// Draws a BubbleView into the markup of index.html. The ring and outline attributes are a
// port of docs/design/mockup/Bubble.dc.html, drawn once per cell.

import { ears } from './ears';
import { APOTHEM, CELL_APOTHEM, CELL_R, FIVE_RING, type RingGeometry, SEVEN_RING, hexPath, ringDash } from './geometry';
import type { BubbleView, CellView, RingView } from './model';

const SVG_NS = 'http://www.w3.org/2000/svg';
const TRACK = '#252836';
const EDGE = 'rgba(255,255,255,0.12)';
const HIGHLIGHT = '#e0e2ea';
const HIGHLIGHT_SW = 1.5;
/** The opacity of a stale cell. */
const STALE_OPACITY = 0.45;

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

function earNodes(view: BubbleView): SVGElement[] {
  const top = view.layout.cells[view.layout.top];
  return ears(view.ears, top.x, view.layout.earBase).flatMap((e) => [
    ...e.facets.map((f) => svg('polygon', { points: f.pts, fill: f.fill })),
    svg('polyline', {
      points: e.edge,
      fill: 'none',
      stroke: EDGE,
      'stroke-width': 1,
      'stroke-linejoin': 'miter'
    })
  ]);
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
    })
  ];
}

function cellNode(c: CellView): SVGElement {
  const g = svg('g', { opacity: c.dim ? STALE_OPACITY : 1 });
  g.append(svg('path', { d: hexPath(CELL_R, c.x, c.y), fill: 'none', stroke: EDGE, 'stroke-width': 1 }));
  if (c.rings) g.append(...ringNodes(c, FIVE_RING, c.rings.five), ...ringNodes(c, SEVEN_RING, c.rings.seven));
  return g;
}

// The highlight runs just inside the cell edge, so it does not cover the next cell.
function highlightNode(c: CellView): SVGElement {
  const r = (CELL_APOTHEM - HIGHLIGHT_SW / 2) / APOTHEM;
  return svg('path', {
    d: hexPath(r, c.x, c.y),
    fill: 'none',
    stroke: HIGHLIGHT,
    'stroke-opacity': 0.7,
    'stroke-width': HIGHLIGHT_SW
  });
}

function labelNode(c: CellView): HTMLElement {
  const el = html('div', 'cell-text');
  el.style.left = c.x + 'px';
  el.style.top = c.y + 'px';
  el.style.transform = 'translate(-50%, calc(-50% + ' + c.fit.offset + 'px))';
  if (c.dim) el.style.opacity = String(STALE_OPACITY);
  el.append(
    html('span', 'countdown', c.text.countdown),
    html('span', 'name', c.fit.name),
    html('span', 'figures', c.text.figures)
  );
  return el;
}

export function renderBubble(root: HTMLElement, view: BubbleView): void {
  const { width, height, cells } = view.layout;
  root.style.width = width + 'px';
  root.style.height = height + 'px';
  root.style.opacity = view.dim ? '0.55' : '1';
  root.setAttribute('aria-label', view.label);
  for (const sel of ['.ears', '.cells']) {
    const el = part<SVGElement>(root, sel);
    el.setAttribute('width', String(width));
    el.setAttribute('height', String(height));
    el.setAttribute('viewBox', '0 0 ' + width + ' ' + height);
  }
  const glass = cells.map((c) => hexPath(CELL_R, c.x, c.y)).join(' ');
  part<HTMLElement>(root, '.glass').style.clipPath = "path('" + glass + "')";
  part<SVGElement>(root, '.ears').replaceChildren(...earNodes(view));
  part<SVGElement>(root, '.cells').replaceChildren(
    ...view.cells.map(cellNode),
    ...view.cells.filter((c) => c.highlight).map(highlightNode)
  );
  part<HTMLElement>(root, '.labels').replaceChildren(...view.cells.map(labelNode));
}
