import '@fontsource/inter/latin-400.css';
import '@fontsource/inter/latin-600.css';
import '@fontsource/fira-code/latin-400.css';
import '@fontsource/fira-code/latin-600.css';
import './style.css';

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

import { renderBubble } from './bubble';
import { cardView } from './card';
import { renderCard } from './cardRender';
import { AUTO, type Pin, keepPin, nextPin } from './centre';
import { parseScript } from './debug';
import { attachPointer } from './drag';
import { bubbleView } from './model';
import type { UsageUpdate } from './usage';

const UPDATE_EVENT = 'usage://update';
// The countdown shows whole minutes, so the page redraws on the same 15 s tick as the mockup.
const REDRAW_MS = 15_000;

function element(id: string): HTMLElement {
  const el = document.getElementById(id);
  if (!el) throw new Error('The page has no #' + id + ' element.');
  return el;
}

const bubble = element('bubble');
const card = element('card');

let latest: UsageUpdate | null = null;
let pin: Pin = AUTO;
let open = false;
let windowSize = '';

// The window size follows the cell count, or the card size when the card is open. The page
// asks the backend to resize only when the size changes. The backend keeps the top-left
// corner in place.
function fitWindow(width: number, height: number): void {
  const size = width + 'x' + height;
  if (size === windowSize) return;
  windowSize = size;
  invoke('set_bubble_size', { width, height }).catch((err) => {
    windowSize = '';
    console.error('usaki: Cannot resize the window.', err);
  });
}

function draw(): void {
  const now = Date.now();
  bubble.hidden = open;
  card.hidden = !open;
  if (open) {
    const view = cardView(latest, now, pin);
    renderCard(card, view);
    fitWindow(view.width, view.height);
  } else {
    const view = bubbleView(latest, now, pin);
    renderBubble(bubble, view);
    fitWindow(view.layout.width, view.layout.height);
  }
}

function accept(update: UsageUpdate): void {
  if (update.error) console.error('usaki: ' + update.error);
  update.accounts
    .filter((a) => a.error)
    .forEach((a) => console.error('usaki: ' + a.error));
  latest = update;
  pin = keepPin(update.accounts, pin);
  draw();
}

function refresh(): void {
  invoke<UsageUpdate>('get_usage')
    .then(accept)
    .catch((err) => console.error('usaki: Cannot get the usage data.', err));
}

function setOpen(value: boolean): void {
  if (open === value) return;
  open = value;
  draw();
  (open ? card : bubble).focus();
}

// The backend watches the window and saves its position when it settles. The page arms it
// when a drag starts and again when it sees the drag end, because the compositor holds the
// pointer during the move and the page may miss the release.
function trackPosition(): void {
  invoke('track_position').catch((err) => console.error('usaki: Cannot track the window position.', err));
}

function action(target: EventTarget | null): string | null {
  return target instanceof Element ? target.closest('[data-action]')?.getAttribute('data-action') ?? null : null;
}

function runAction(name: string | null): void {
  if (name === 'open') setOpen(true);
  else if (name === 'close') setOpen(false);
  else if (name === 'refresh') refresh();
}

attachPointer(document.body, {
  isHandle: (t) => t instanceof Element && t.closest('[data-drag]') != null,
  startDrag: () => {
    trackPosition();
    return getCurrentWindow().startDragging();
  },
  dragEnd: trackPosition,
  click: (t) => runAction(action(t))
});

// A key press on a button gives a click with detail 0. A pointer click already ran its
// action through the gesture, so it is skipped here.
document.addEventListener('click', (e) => {
  if (e.detail === 0) runAction(action(e.target));
});

// Right-click pins the next account in the head.
function pinNext(): void {
  if (open || !latest) return;
  pin = nextPin(latest.accounts, pin, Date.now());
  draw();
}

// Middle-click returns the head to auto.
function toAuto(): void {
  if (open || pin === AUTO) return;
  pin = AUTO;
  draw();
}

// The page also blocks the webview menu.
window.addEventListener('contextmenu', (e) => {
  e.preventDefault();
  pinNext();
});

window.addEventListener('auxclick', (e) => {
  if (e.button !== 1) return;
  e.preventDefault();
  toAuto();
});

window.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && open) setOpen(false);
  else if ((e.key === 'Enter' || e.key === ' ') && !open && document.activeElement === bubble) setOpen(true);
});

draw();
setInterval(draw, REDRAW_MS);

listen<UsageUpdate>(UPDATE_EVENT, (e) => accept(e.payload)).catch((err) =>
  console.error('usaki: Cannot listen for usage updates.', err)
);
refresh();

// A debug build can run a script of the mouse actions, for checks without a mouse. The
// script calls the same functions as the mouse handlers. A release build sends no script.
const DEBUG_STEP_MS = 3_000;
const DEBUG_ACTIONS = { right: pinNext, middle: toAuto, open: () => setOpen(true), close: () => setOpen(false) };
invoke<string | null>('debug_ui')
  .then((script) =>
    parseScript(script).forEach((a, i) => setTimeout(DEBUG_ACTIONS[a], (i + 1) * DEBUG_STEP_MS))
  )
  .catch((err) => console.error('usaki: Cannot read the debug script.', err));
