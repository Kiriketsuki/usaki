import { describe, expect, it } from 'vitest';
import { BLONDE, EMERALD, RUBY } from './colour';
import { NOW, account } from './test-fixtures';
import { bubbleView } from './model';

const update = (accounts: ReturnType<typeof account>[]) => ({ polledAt: NOW, accounts, error: null });

describe('bubbleView', () => {
  it('draws one ring pair per account in order, each ring coloured by its own window', () => {
    const view = bubbleView(update([account('acc1', 42, 133, false, 63), account('acc2', 81, 47, false, 20)]), NOW);
    expect(view.pairs).toEqual([
      { five: { pct: 42, color: EMERALD }, seven: { pct: 63, color: BLONDE }, dim: false },
      { five: { pct: 81, color: RUBY }, seven: { pct: 20, color: EMERALD }, dim: false }
    ]);
    expect(view.ears).toBe('one');
    expect(view.dim).toBe(false);
    expect(view.fit.sub).toBe('acc2 81/20%');
  });

  it('dims both rings of a stale account and keeps its ears out of the rule', () => {
    const view = bubbleView(update([account('acc1', 42, 133), account('acc2', 81, 47, true)]), NOW);
    expect(view.pairs[1].dim).toBe(true);
    expect(view.ears).toBe('up');
    expect(view.fit.sub).toBe('acc1 42/10%');
  });

  it('keeps the ears on the 5h rings, so a high 7d window folds no ear', () => {
    const view = bubbleView(update([account('acc1', 42, 133, false, 90)]), NOW);
    expect(view.ears).toBe('up');
  });

  it('dims the whole bubble and folds both ears when no account is live', () => {
    const view = bubbleView(update([account('acc1', 42, 133, true)]), NOW);
    expect(view.dim).toBe(true);
    expect(view.ears).toBe('both');
    expect(view.text.top).toBe('no data');
  });

  it('shows no data before the first update', () => {
    const view = bubbleView(null, NOW);
    expect(view.pairs).toEqual([]);
    expect(view.dim).toBe(true);
    expect(view.label).toBe('AI usage. No fresh data.');
  });

  it('shows 0% for a window whose reset passed after the poll', () => {
    const view = bubbleView(update([account('acc1', 90, -1)]), NOW);
    expect(view.pairs[0].five.pct).toBe(0);
  });

  it('cuts a long name on screen but keeps it whole in the label', () => {
    const accts = ['longname12ch', 'acc2', 'acc3', 'acc4', 'acc5'].map((id, i) =>
      account(id, 81, 47 + i * 10, false, 58)
    );
    const view = bubbleView(update(accts), NOW);
    expect(view.fit.sub).toMatch(/^longna\w*\u2026 81\/58%$/);
    expect(view.label).toBe(
      'AI usage. longname12ch: 81% used in the 5 hour window, 58% used in the 7 day window. ' +
        'The 5 hour window resets in 47m.'
    );
  });
});
