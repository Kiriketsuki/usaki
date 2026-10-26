// A left-drag moves the window and a left-click runs the action under the pointer. The drag
// starts only after the pointer moves past the threshold, so a drag never counts as a click.

/** The drag threshold from Main.dc.html: the sum of |dx| and |dy|, in CSS px. */
export const DRAG_THRESHOLD = 4;

export function passedThreshold(dx: number, dy: number): boolean {
  return Math.abs(dx) + Math.abs(dy) >= DRAG_THRESHOLD;
}

/**
 * The state of one left press. `pressed` waits for the threshold. `dragging` means the
 * compositor moves the window. The compositor holds the pointer during a move, so the page
 * may not see the release. The first pointer event with no button down then ends the drag.
 */
export type Gesture =
  | { kind: 'idle' }
  | { kind: 'pressed'; x: number; y: number }
  | { kind: 'dragging' };

/** What the page must do after a pointer event. */
export type Effect = 'start-drag' | 'drag-end' | 'click' | null;

export interface Step {
  gesture: Gesture;
  effect: Effect;
}

export const IDLE: Gesture = { kind: 'idle' };

/**
 * A press. Only a left press on a drag handle starts a gesture. A press that comes while a
 * drag still runs ends that drag first, because the page missed its release.
 */
export function press(g: Gesture, button: number, onHandle: boolean, x: number, y: number): Step {
  const next: Gesture = button === 0 && onHandle ? { kind: 'pressed', x, y } : IDLE;
  return { gesture: next, effect: g.kind === 'dragging' ? 'drag-end' : null };
}

/** A pointer move. `buttons` is the PointerEvent bit mask of the buttons that are down. */
export function move(g: Gesture, x: number, y: number, buttons: number): Step {
  if (g.kind === 'dragging') return buttons === 0 ? { gesture: IDLE, effect: 'drag-end' } : { gesture: g, effect: null };
  if (g.kind === 'pressed' && passedThreshold(x - g.x, y - g.y)) {
    return { gesture: { kind: 'dragging' }, effect: 'start-drag' };
  }
  return { gesture: g, effect: null };
}

/** A release. A press that never passed the threshold is a click. */
export function release(g: Gesture): Step {
  if (g.kind === 'pressed') return { gesture: IDLE, effect: 'click' };
  if (g.kind === 'dragging') return { gesture: IDLE, effect: 'drag-end' };
  return { gesture: IDLE, effect: null };
}

/** A cancel, for example when the compositor takes the pointer. A running drag stays open. */
export function cancel(g: Gesture): Step {
  return { gesture: g.kind === 'dragging' ? g : IDLE, effect: null };
}

export interface PointerHandlers {
  /** Returns true when the press target is a drag handle. */
  isHandle: (target: EventTarget | null) => boolean;
  startDrag: () => Promise<void>;
  dragEnd: () => void;
  /** Runs the click action of the press target. */
  click: (target: EventTarget | null) => void;
}

/** Wires the gesture to the pointer events of `el`. */
export function attachPointer(el: HTMLElement, h: PointerHandlers): void {
  let g: Gesture = IDLE;
  let target: EventTarget | null = null;
  const run = (step: Step) => {
    g = step.gesture;
    if (step.effect === 'start-drag') {
      h.startDrag().catch((err) => {
        g = IDLE;
        console.error('usaki: Cannot start the window drag.', err);
      });
    } else if (step.effect === 'drag-end') {
      h.dragEnd();
    } else if (step.effect === 'click') {
      h.click(target);
    }
  };
  el.addEventListener('pointerdown', (e) => {
    // A middle press starts the autoscroll or the paste before auxclick fires.
    if (e.button === 1) e.preventDefault();
    target = e.target;
    run(press(g, e.button, h.isHandle(e.target), e.clientX, e.clientY));
  });
  el.addEventListener('pointermove', (e) => run(move(g, e.clientX, e.clientY, e.buttons)));
  el.addEventListener('pointerover', (e) => run(move(g, e.clientX, e.clientY, e.buttons)));
  el.addEventListener('pointerup', () => run(release(g)));
  el.addEventListener('pointercancel', () => run(cancel(g)));
}
