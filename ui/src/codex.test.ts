// The rules for a 7d-only account, such as Codex on the Pro plan. Synthetic data only.

import { describe, expect, it } from 'vitest';
import { CARD_ROW, blockHeight, cardView, creditsText } from './card';
import { AUTO, cellText, keepPin, nextPin, pickAuto, pickHead } from './centre';
import { markPoints } from './marks';
import { bubbleView } from './model';
import { MIN, NOW, account, codexAccount } from './test-fixtures';
import { type AccountView, weekOnly } from './usage';

const update = (accounts: AccountView[]) => ({ polledAt: NOW, accounts, error: null });
const WEEK = 4 * 1440 + 6 * 60;

describe('weekOnly', () => {
  it('is true for a 7d window with no 5h window only', () => {
    expect(weekOnly(codexAccount('codex', 37, WEEK))).toBe(true);
    expect(weekOnly(account('acc1', 10, 60))).toBe(false);
    expect(weekOnly({ ...account('acc1', 10, 60), windows: [] })).toBe(false);
  });
});

describe('cellText of a 7d-only account', () => {
  it('shows -- in both 5h slots, the mark and the name', () => {
    expect(cellText(codexAccount('codex', 37, WEEK), NOW)).toEqual({
      provider: 'codex',
      name: 'codex',
      five: { pct: '--', countdown: '--' },
      seven: { pct: '37%', countdown: '4d06h' }
    });
  });

  it('shows 0% and ready once the 7d reset has passed', () => {
    expect(cellText(codexAccount('codex', 37, -5), NOW).seven).toEqual({ pct: '0%', countdown: 'ready' });
  });
});

describe('head rule', () => {
  it('ranks a 7d-only account after every live account with a 5h window', () => {
    const accts = [account('acc1', 10, 200, false, 10, 3 * 1440), codexAccount('codex', 37, 30)];
    expect(pickAuto(accts, NOW)?.id).toBe('acc1');
  });

  it('ranks a 7d-only account after an account with an unused 5h window', () => {
    const accts = [codexAccount('codex', 37, 30), account('acc1', 0, null)];
    expect(pickAuto(accts, NOW)?.id).toBe('acc1');
  });

  it('makes a 7d-only account the head when no live account has a 5h window', () => {
    const accts = [account('acc1', 10, 20, true), codexAccount('late', 5, WEEK), codexAccount('soon', 50, 90)];
    expect(pickHead(accts, AUTO, NOW)).toEqual({ account: accts[2], mode: 'auto' });
  });

  it('lets a pin make a 7d-only account the head', () => {
    const accts = [account('acc1', 10, 20), codexAccount('codex', 37, WEEK)];
    expect(nextPin(accts, AUTO, NOW)).toBe('codex:codex');
    expect(pickHead(accts, 'codex:codex', NOW).account).toBe(accts[1]);
  });

  it('keeps a Claude pin and a Codex pin with the same name apart', () => {
    const claude = { ...account('main', 10, 20), id: 'claude:main', name: 'main' };
    const accts = [claude, codexAccount('main', 37, WEEK)];
    expect(pickHead(accts, 'codex:main', NOW).account?.provider).toBe('codex');
    expect(pickHead(accts, 'claude:main', NOW).account?.provider).toBe('claude');
    expect(keepPin(accts, 'codex:main')).toBe('codex:main');
    expect(keepPin(accts, 'main')).toBe(AUTO);
  });
});

describe('bubble of a 7d-only account', () => {
  it('has no 5h ring and a 7d ring', () => {
    const view = bubbleView(update([codexAccount('codex', 37, WEEK)]), NOW);
    expect(view.cells[0].rings).toEqual({ five: null, seven: { pct: 37, color: '#1a8a6a' } });
    expect(view.cells[0].fit.mark).not.toBeNull();
  });

  it('says the provider and no 5 hour window in the label', () => {
    const view = bubbleView(update([codexAccount('codex', 37, WEEK)]), NOW);
    expect(view.label.startsWith('AI usage. Next reset: Codex codex. 5 hour window: none. 7 day window: 37% used')).toBe(
      true
    );
  });

  it('counts for the left ear only', () => {
    const high = codexAccount('codex', 80, WEEK);
    expect(bubbleView(update([account('acc1', 10, 20), high]), NOW).ears).toBe('left');
    expect(bubbleView(update([account('acc1', 10, 20), codexAccount('codex', 20, WEEK)]), NOW).ears).toBe('up');
    expect(bubbleView(update([codexAccount('codex', 100, WEEK)]), NOW).ears).toBe('both');
  });

  it('keeps the account order in the body: Claude first, then Codex', () => {
    const accts = [account('acc1', 10, 200), account('acc2', 10, 20), codexAccount('codex', 37, WEEK)];
    expect(bubbleView(update(accts), NOW).cells.map((c) => c.id)).toEqual(['acc2', 'acc1', 'codex:codex']);
  });
});

describe('card of a 7d-only account', () => {
  it('has a teal dot, the 7d row, the credits row and no 5h row', () => {
    const view = cardView(update([account('acc1', 10, 20), codexAccount('codex', 37, WEEK)]), NOW, AUTO);
    const [claude, codex] = view.rows;
    expect(claude).toMatchObject({ name: 'acc1', dot: '#7a4fb8', credits: null, height: CARD_ROW });
    expect(codex).toMatchObject({ id: 'codex:codex', name: 'codex', dot: '#20969c', credits: '123.45' });
    expect(codex.windows.map((w) => w.label)).toEqual(['7d']);
    expect(codex.height).toBe(blockHeight(2));
  });

  it('tags the 7d-only head next 7d', () => {
    const view = cardView(update([account('acc1', 10, 20, true), codexAccount('codex', 37, WEEK)]), NOW, AUTO);
    expect(view.rows.map((r) => r.tag)).toEqual([null, 'next 7d']);
  });

  it('gives a shorter block with no credits', () => {
    const view = cardView(update([codexAccount('codex', 37, WEEK, false, null)]), NOW, AUTO);
    expect(view.rows[0].height).toBe(blockHeight(1));
  });

  it('shows the balance as given, or unlimited', () => {
    expect(creditsText({ hasCredits: true, unlimited: false, balance: '21.5000' })).toBe('21.5000');
    expect(creditsText({ hasCredits: true, unlimited: true, balance: null })).toBe('unlimited');
    expect(creditsText({ hasCredits: false, unlimited: false, balance: null })).toBeNull();
    expect(creditsText(null)).toBeNull();
  });

  it('shows a stale 7d-only account with its age', () => {
    const old = { ...codexAccount('codex', 37, -5, true), fetchedAt: NOW - 8 * 60 * MIN };
    expect(cardView(update([old]), NOW, AUTO).rows[0].stale).toBe('stale 8h');
  });
});

describe('markPoints', () => {
  it('gives a 16 point starburst and a 6 point chevron inside the box', () => {
    const parse = (s: string) => s.split(' ').map((p) => p.split(',').map(Number));
    const star = parse(markPoints('claude', 9));
    const chevron = parse(markPoints('codex', 9));
    expect(star).toHaveLength(16);
    expect(chevron).toHaveLength(6);
    for (const [x, y] of [...star, ...chevron]) {
      expect(x).toBeGreaterThanOrEqual(0);
      expect(x).toBeLessThanOrEqual(9);
      expect(y).toBeGreaterThanOrEqual(0);
      expect(y).toBeLessThanOrEqual(9);
    }
  });
});
