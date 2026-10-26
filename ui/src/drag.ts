// A left-drag moves the window. The drag starts only after the pointer moves past the
// threshold, so a plain click stays free for the card in milestone 2.

/** The drag threshold from Main.dc.html: the sum of |dx| and |dy|, in CSS px. */
export const DRAG_THRESHOLD = 4;

export function passedThreshold(dx: number, dy: number): boolean {
  return Math.abs(dx) + Math.abs(dy) >= DRAG_THRESHOLD;
}

/** Calls `start` once per press, when a left press moves past the threshold. */
export function attachDrag(el: HTMLElement, start: () => Promise<void>): void {
  let origin: { x: number; y: number } | null = null;
  el.addEventListener('pointerdown', (e) => {
    origin = e.button === 0 ? { x: e.clientX, y: e.clientY } : null;
  });
  el.addEventListener('pointermove', (e) => {
    if (!origin || !passedThreshold(e.clientX - origin.x, e.clientY - origin.y)) return;
    origin = null;
    start().catch((err) => console.error('usaki: Cannot start the window drag.', err));
  });
  const end = () => {
    origin = null;
  };
  el.addEventListener('pointerup', end);
  el.addEventListener('pointercancel', end);
}
