import { describe, expect, it } from 'vitest';
import { EMERALD, RUBY } from './colour';
import { NOW, account } from './test-fixtures';
import { bubbleView } from './model';

const update = (accounts: ReturnType<typeof account>[]) => ({ polledAt: NOW, accounts, error: null });

describe('bubbleView', () => {
  it('draws one ring per account in order, coloured by the 5h percent', () => {
    const view = bubbleView(update([account('acc1', 42, 133), account('acc2', 81, 47)]), NOW);
    expect(view.rings).toEqual([
      { pct: 42, color: EMERALD, dim: false },
      { pct: 81, color: RUBY, dim: false }
    ]);
    expect(view.ears).toBe('one');
    expect(view.dim).toBe(false);
    expect(view.text.sub).toBe('acc2 · 81%');
  });

  it('dims the ring of a stale account and keeps its ears out of the rule', () => {
    const view = bubbleView(update([account('acc1', 42, 133), account('acc2', 81, 47, true)]), NOW);
    expect(view.rings[1].dim).toBe(true);
    expect(view.ears).toBe('up');
    expect(view.text.sub).toBe('acc1 · 42%');
  });

  it('dims the whole bubble and folds both ears when no account is live', () => {
    const view = bubbleView(update([account('acc1', 42, 133, true)]), NOW);
    expect(view.dim).toBe(true);
    expect(view.ears).toBe('both');
    expect(view.text.top).toBe('no data');
  });

  it('shows no data before the first update', () => {
    const view = bubbleView(null, NOW);
    expect(view.rings).toEqual([]);
    expect(view.dim).toBe(true);
    expect(view.label).toBe('AI usage. No fresh data.');
  });

  it('shows 0% for a window whose reset passed after the poll', () => {
    const view = bubbleView(update([account('acc1', 90, -1)]), NOW);
    expect(view.rings[0].pct).toBe(0);
  });
});
