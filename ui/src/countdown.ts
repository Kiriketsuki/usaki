// Countdown text, ported from left() in docs/design/mockup/Main.dc.html.

const MINUTE_MS = 60_000;

function pad(n: number): string {
  return String(n).padStart(2, '0');
}

/** Formats minutes left as `now`, `47m`, `2h 13m` or `4d 6h`. */
export function left(mins: number): string {
  if (mins <= 0) return 'now';
  const m = Math.ceil(mins);
  const d = Math.floor(m / 1440);
  const h = Math.floor((m % 1440) / 60);
  const mm = m % 60;
  if (d > 0) return d + 'd ' + h + 'h';
  if (h > 0) return h + 'h ' + pad(mm) + 'm';
  return mm + 'm';
}

/** Returns the minutes from `now` to `at`. Both are epoch ms. */
export function minutesUntil(at: number, now: number): number {
  return (at - now) / MINUTE_MS;
}
