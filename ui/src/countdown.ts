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

/**
 * Formats minutes left as the card shows them: `now`, `47m`, `2h 13m` or `4d 6h`. This is
 * the left() of docs/design/mockup/Main.dc.html. The card has room for the spaces.
 */
export function spacedLeft(mins: number): string {
  if (mins <= 0) return 'now';
  const m = Math.ceil(mins);
  const d = Math.floor(m / 1440);
  const h = Math.floor((m % 1440) / 60);
  const mm = m % 60;
  if (d > 0) return d + 'd ' + h + 'h';
  if (h > 0) return h + 'h ' + pad(mm) + 'm';
  return mm + 'm';
}

/**
 * Formats the clock time of `at` (epoch ms) as `16:40`, or as `Tue 20:47` with the day. The
 * time is local unless the caller gives an IANA time zone, as the tests do.
 */
export function clockAt(at: number, withDay: boolean, timeZone?: string): string {
  const parts = new Intl.DateTimeFormat('en-GB', {
    weekday: 'short',
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
    timeZone
  }).formatToParts(new Date(at));
  const part = (type: string) => parts.find((p) => p.type === type)?.value ?? '';
  const hm = part('hour') + ':' + part('minute');
  return withDay ? part('weekday') + ' ' + hm : hm;
}

/** Formats an age in ms as a coarse `12m`, `3h` or `2d`. An age under a minute gives ''. */
export function ageText(ms: number): string {
  const mins = Math.floor(ms / MINUTE_MS);
  if (mins < 1) return '';
  if (mins < 60) return mins + 'm';
  const hours = Math.floor(mins / 60);
  if (hours < 48) return hours + 'h';
  return Math.floor(hours / 24) + 'd';
}
