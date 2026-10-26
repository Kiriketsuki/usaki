// Draws a BubbleView into the markup of index.html. The markup and the attribute values
// are a port of docs/design/mockup/Bubble.dc.html.

import { ears } from './ears';
import { OUTLINE_R, type RingGeometry, hexPath, ringDash, ringPair } from './geometry';
import type { BubbleView, RingView } from './model';

const SVG_NS = 'http://www.w3.org/2000/svg';
const TRACK = '#252836';
const EAR_EDGE = 'rgba(255,255,255,0.12)';

function svg(tag: string, attrs: Record<string, string | number>): SVGElement {
  const el = document.createElementNS(SVG_NS, tag);
  Object.entries(attrs).forEach(([k, v]) => el.setAttribute(k, String(v)));
  return el;
}

function part<T extends Element>(root: ParentNode, selector: string): T {
  const el = root.querySelector<T>(selector);
  if (!el) throw new Error('The bubble markup has no ' + selector + ' element.');
  return el;
}

function earNodes(view: BubbleView): SVGElement[] {
  return ears(view.ears).flatMap((e) => [
    ...e.facets.map((f) => svg('polygon', { points: f.pts, fill: f.fill })),
    svg('polyline', {
      points: e.edge,
      fill: 'none',
      stroke: EAR_EDGE,
      'stroke-width': 1,
      'stroke-linejoin': 'miter'
    })
  ]);
}

// A ring is a track path and a progress path on top of it, as in the mockup.
function ringNode(g: RingGeometry, ring: RingView, dim: boolean): SVGElement[] {
  return [
    svg('path', { d: g.d, fill: 'none', stroke: TRACK, 'stroke-width': g.sw }),
    svg('path', {
      d: g.d,
      fill: 'none',
      stroke: ring.color,
      'stroke-width': g.sw,
      'stroke-opacity': dim ? 0.35 : 1,
      pathLength: 100,
      'stroke-dasharray': ringDash(ring.pct),
      'stroke-linecap': 'butt'
    })
  ];
}

function ringNodes(view: BubbleView): SVGElement[] {
  return view.pairs.flatMap((pair, i) => {
    const g = ringPair(i, view.pairs.length);
    return [...ringNode(g.five, pair.five, pair.dim), ...ringNode(g.seven, pair.seven, pair.dim)];
  });
}

function setLine(root: ParentNode, selector: string, text: string, size: number): void {
  const el = part<HTMLElement>(root, selector);
  el.textContent = text;
  el.style.fontSize = size + 'px';
}

export function renderBubble(root: HTMLElement, view: BubbleView): void {
  const { sizes, sub, offset } = view.fit;
  root.style.opacity = view.dim ? '0.55' : '1';
  root.setAttribute('aria-label', view.label);
  part<SVGElement>(root, '.outline').setAttribute('d', hexPath(OUTLINE_R));
  part<SVGElement>(root, '.ears').replaceChildren(...earNodes(view));
  part<SVGElement>(root, '.ring-list').replaceChildren(...ringNodes(view));
  part<HTMLElement>(root, '.text').style.transform = 'translateY(' + offset + 'px)';
  setLine(root, '.top', view.text.top, sizes.top);
  setLine(root, '.centre', view.text.centre, sizes.centre);
  setLine(root, '.sub', sub, sizes.sub);
}
