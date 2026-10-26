import { describe, expect, it } from 'vitest';
import { CARD_BAR, CARD_HEADER, CARD_ROW, CARD_WIDTH, cardSize, cardView, staleText, updatedText, windowRow } from './card';
import { AUTO } from './centre';
import { BLONDE, EMERALD, RUBY } from './colour';
import { MIN, NOW, account } from './test-fixtures';
import type { AccountView } from './usage';

const UTC = 'UTC';
const update = (accounts: AccountView[]) => ({ polledAt: NOW, accounts, error: null });

describe('cardSize', () => {
  it('holds the bar, the header and one block per account', () => {
    expect(cardSize([CARD_ROW, CARD_ROW, 65, 109])).toEqual({
      width: CARD_WIDTH,
      height: CARD_BAR + CARD_HEADER + 2 * CARD_ROW + 65 + 109
    });
  });

  it('keeps one block for the no data text', () => {
    expect(cardSize([])).toEqual(cardSize([CARD_ROW]));
  });
});

describe('windowRow', () => {
  // NOW is a Monday at 12:00 UTC.
  const a = account('acc1', 42, 133, false, 58, 4 * 1440 + 6 * 60 + 20);

  it('shows the 5h percent, bar, countdown and clock time, with no moon', () => {
    expect(windowRow(a, 300, NOW, undefined, UTC)).toEqual({
      label: '5h', bar: 42, pct: '42%', color: EMERALD, left: '2h 13m', at: '14:13', moon: null
    });
  });

  it('shows the 7d reset with the weekday and a waxing moon', () => {
    const row = windowRow(a, 10080, NOW, undefined, UTC);
    expect(row).toMatchObject({ label: '7d', bar: 58, pct: '58%', color: BLONDE, left: '4d 6h', at: 'Fri 18:20' });
    expect(row.moon?.label).toBe('Moon 39% full, 7d reset in 4d 6h');
    expect(row.moon?.lit).toMatch(/^M 6 1 A 5 5 0 0 1 6 11 A /);
  });

  it('shows ready and a new moon for windows with no countdown', () => {
    const idle = account('acc1', 90, -1, false, 30, null);
    expect(windowRow(idle, 300, NOW, undefined, UTC)).toMatchObject({ pct: '0%', bar: 0, left: 'ready', at: '' });
    expect(windowRow(idle, 10080, NOW, undefined, UTC)).toMatchObject({ left: 'ready', at: '', moon: { lit: '' } });
  });

  it('shows -- for a missing window and clamps the bar', () => {
    const empty: AccountView = { ...a, windows: [] };
    expect(windowRow(empty, 300, NOW)).toMatchObject({ pct: '--', left: '--', bar: 0, moon: null });
    const over = account('acc1', 130, 10);
    expect(windowRow(over, 300, NOW)).toMatchObject({ pct: '130%', bar: 100, color: RUBY });
  });
});

describe('staleText and updatedText', () => {
  it('gives the age of stale data, and null for a live account', () => {
    expect(staleText(account('acc1', 1, 10), NOW)).toBeNull();
    const old = { ...account('acc1', 1, 10, true), fetchedAt: NOW - 3 * 60 * MIN };
    expect(staleText(old, NOW)).toBe('stale 3h');
    expect(staleText({ ...old, fetchedAt: NOW - 20 * MIN }, NOW)).toBe('stale 20m');
    expect(staleText({ ...old, fetchedAt: NOW - 3 * 1440 * MIN }, NOW)).toBe('stale 3d');
    expect(staleText({ ...old, fetchedAt: NOW }, NOW)).toBe('stale');
  });

  it('dates the card from the newest data that was read', () => {
    const accts = [
      { ...account('acc1', 1, 10), fetchedAt: NOW - 5 * MIN },
      { ...account('acc2', 1, 10), fetchedAt: NOW - 2 * MIN },
      { ...account('acc3', 1, 10, true), fetchedAt: NOW, error: 'unreadable' }
    ];
    expect(updatedText(accts, NOW)).toBe('updated 2m ago');
    expect(updatedText([account('acc1', 1, 10)], NOW)).toBe('updated now');
    expect(updatedText([], NOW)).toBe('no data');
  });
});

describe('cardView', () => {
  const accts = [account('acc1', 42, 133), account('acc2', 81, 47), account('acc3', 5, 90, true)];

  it('keeps the account order and tags the auto head', () => {
    const view = cardView(update(accts), NOW, AUTO, undefined, UTC);
    expect(view.rows.map((r) => [r.id, r.tag, r.stale])).toEqual([
      ['acc1', null, null],
      ['acc2', 'next 5h', null],
      ['acc3', null, 'stale']
    ]);
    expect(view.rows.map((r) => r.windows.map((w) => w.label))).toEqual([['5h', '7d'], ['5h', '7d'], ['5h', '7d']]);
    expect(view.rows.map((r) => r.height)).toEqual([CARD_ROW, CARD_ROW, CARD_ROW]);
    expect([view.width, view.height]).toEqual([CARD_WIDTH, cardSize([CARD_ROW, CARD_ROW, CARD_ROW]).height]);
  });

  it('tags a pinned head', () => {
    const view = cardView(update(accts), NOW, 'acc3', undefined, UTC);
    expect(view.rows.map((r) => r.tag)).toEqual([null, null, 'pinned']);
  });

  it('gives no rows and the one block size with no data', () => {
    const view = cardView(null, NOW, AUTO);
    expect(view.rows).toEqual([]);
    expect(view.updated).toBe('no data');
    expect(view.height).toBe(cardSize([]).height);
  });
});
