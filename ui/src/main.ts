import '@fontsource/inter/latin-400.css';
import '@fontsource/fira-code/latin-400.css';
import '@fontsource/fira-code/latin-600.css';
import './style.css';

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

import { renderBubble } from './bubble';
import { attachDrag } from './drag';
import { bubbleView } from './model';
import type { UsageUpdate } from './usage';

const UPDATE_EVENT = 'usage://update';
// The countdown shows whole minutes, so the page redraws on the same 15 s tick as the mockup.
const REDRAW_MS = 15_000;

const root = document.getElementById('bubble');
if (!root) throw new Error('The page has no #bubble element.');
const bubble: HTMLElement = root;

let latest: UsageUpdate | null = null;
let windowSize = '';

// The window size follows the cell count. The page asks the backend to resize only when
// the size changes. The backend keeps the top-left corner in place.
function fitWindow(width: number, height: number): void {
  const size = width + 'x' + height;
  if (size === windowSize) return;
  windowSize = size;
  invoke('set_bubble_size', { width, height }).catch((err) => {
    windowSize = '';
    console.error('usaki: Cannot resize the bubble window.', err);
  });
}

function draw(): void {
  const view = bubbleView(latest, Date.now());
  renderBubble(bubble, view);
  fitWindow(view.layout.width, view.layout.height);
}

function accept(update: UsageUpdate): void {
  if (update.error) console.error('usaki: ' + update.error);
  update.accounts
    .filter((a) => a.error)
    .forEach((a) => console.error('usaki: ' + a.error));
  latest = update;
  draw();
}

// Right-click cycling comes in milestone 2. Until then the page blocks the webview menu.
window.addEventListener('contextmenu', (e) => e.preventDefault());
attachDrag(bubble, () => getCurrentWindow().startDragging());
draw();
setInterval(draw, REDRAW_MS);

listen<UsageUpdate>(UPDATE_EVENT, (e) => accept(e.payload)).catch((err) =>
  console.error('usaki: Cannot listen for usage updates.', err)
);
invoke<UsageUpdate>('get_usage')
  .then(accept)
  .catch((err) => console.error('usaki: Cannot get the first usage data.', err));
