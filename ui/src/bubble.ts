// Draws a BubbleView into the markup of index.html. The markup and the attribute values
// are a port of docs/design/mockup/Bubble.dc.html.

import { ears } from './ears';
import { OUTLINE_R, hexPath, ringDash, ringGeometry } from './geometry';
import type { BubbleView } from './model';

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

function ringNodes(view: BubbleView): SVGElement[] {
  return view.rings.flatMap((ring, i) => {
    const g = ringGeometry(i, view.rings.length);
    return [
      svg('path', { d: g.d, fill: 'none', stroke: TRACK, 'stroke-width': g.sw }),
      svg('path', {
        d: g.d,
        fill: 'none',
        stroke: ring.color,
        'stroke-width': g.sw,
        'stroke-opacity': ring.dim ? 0.35 : 1,
        pathLength: 100,
        'stroke-dasharray': ringDash(ring.pct),
        'stroke-linecap': 'butt'
      })
    ];
  });
}

export function renderBubble(root: HTMLElement, view: BubbleView): void {
  root.style.opacity = view.dim ? '0.55' : '1';
  root.setAttribute('aria-label', view.label);
  part<SVGElement>(root, '.outline').setAttribute('d', hexPath(OUTLINE_R));
  part<SVGElement>(root, '.ears').replaceChildren(...earNodes(view));
  part<SVGElement>(root, '.ring-list').replaceChildren(...ringNodes(view));
  part<HTMLElement>(root, '.top').textContent = view.text.top;
  part<HTMLElement>(root, '.centre').textContent = view.text.centre;
  part<HTMLElement>(root, '.sub').textContent = view.text.sub;
}
