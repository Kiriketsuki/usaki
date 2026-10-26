// Countdown text. The mockup left() in docs/design/mockup/Main.dc.html set the idea. Issue
// #11 made the text compact, so a 5h and a 7d countdown fit side by side in one cell.

const MINUTE_MS = 60_000;

function pad(n: number): string {
  return String(n).padStart(2, '0');
}

/** Formats minutes left as `now`, `33m`, `1h12m` or `2d04h`. It rounds up to whole minutes. */
export function left(mins: number): string {
  if (mins <= 0) return 'now';
  const m = Math.ceil(mins);
  const d = Math.floor(m / 1440);
  const h = Math.floor((m % 1440) / 60);
  const mm = m % 60;
  if (d > 0) return d + 'd' + pad(h) + 'h';
  if (h > 0) return h + 'h' + pad(mm) + 'm';
  return mm + 'm';
}

/** Returns the minutes from `now` to `at`. Both are epoch ms. */
export function minutesUntil(at: number, now: number): number {
  return (at - now) / MINUTE_MS;
}
