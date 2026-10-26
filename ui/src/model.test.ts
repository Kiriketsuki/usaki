import { describe, expect, it } from 'vitest';
import { BLONDE, EMERALD, RUBY } from './colour';
import { layout } from './layout';
import { NOW, account } from './test-fixtures';
import { bubbleView } from './model';

const update = (accounts: ReturnType<typeof account>[]) => ({ polledAt: NOW, accounts, error: null });

describe('bubbleView', () => {
  it('draws one cell per account in order, each ring coloured by its own window', () => {
    const view = bubbleView(update([account('acc1', 42, 133, false, 63), account('acc2', 81, 47, false, 20)]), NOW);
    expect(view.cells.map((c) => c.rings)).toEqual([
      { five: { pct: 42, color: EMERALD }, seven: { pct: 63, color: BLONDE } },
      { five: { pct: 81, color: RUBY }, seven: { pct: 20, color: EMERALD } }
    ]);
    expect(view.cells.map((c) => c.text.name)).toEqual(['acc1', 'acc2']);
    expect(view.cells[1].text).toEqual({
      name: 'acc2',
      five: { pct: '81%', countdown: '47m' },
      seven: { pct: '20%', countdown: '3d00h' }
    });
    expect(view.ears).toBe('right');
    expect(view.dim).toBe(false);
  });

  it('places the cells on the layout for the account count', () => {
    const view = bubbleView(update([account('acc1', 1, 1), account('acc2', 1, 2), account('acc3', 1, 3)]), NOW);
    expect(view.layout).toEqual(layout(3));
    view.cells.forEach((c, i) => expect([c.x, c.y]).toEqual([layout(3).cells[i].x, layout(3).cells[i].y]));
  });

  it('highlights only the auto account, the next 5h reset', () => {
    const view = bubbleView(update([account('acc1', 42, 133), account('acc2', 81, 47), account('acc3', 5, 90)]), NOW);
    expect(view.cells.map((c) => c.highlight)).toEqual([false, true, false]);
  });

  it('dims a stale cell, keeps it out of the highlight and out of the ear rule', () => {
    const view = bubbleView(update([account('acc1', 42, 133), account('acc2', 81, 47, true)]), NOW);
    expect(view.cells.map((c) => c.dim)).toEqual([false, true]);
    expect(view.cells.map((c) => c.highlight)).toEqual([true, false]);
    expect(view.ears).toBe('up');
  });

  it('folds the left ear for a high 7d window', () => {
    expect(bubbleView(update([account('acc1', 42, 133, false, 90)]), NOW).ears).toBe('left');
  });

  it('keeps a stale account out of the 7d ear rule', () => {
    const accts = [account('acc1', 42, 133), account('acc2', 20, 47, true, 95)];
    expect(bubbleView(update(accts), NOW).ears).toBe('up');
  });

  it('dims the whole bubble and folds both ears when no account is live', () => {
    const view = bubbleView(update([account('acc1', 42, 133, true)]), NOW);
    expect(view.dim).toBe(true);
    expect(view.ears).toBe('both');
    expect(view.cells.some((c) => c.highlight)).toBe(false);
  });

  it('shows one no data cell before the first update', () => {
    const view = bubbleView(null, NOW);
    expect(view.cells).toHaveLength(1);
    expect(view.cells[0].rings).toBeNull();
    expect(view.cells[0].text.name).toBe('no data');
    expect(view.dim).toBe(true);
    expect(view.label).toBe('AI usage. No fresh data.');
  });

  it('shows 0% for a window whose reset passed after the poll', () => {
    const view = bubbleView(update([account('acc1', 90, -1)]), NOW);
    expect(view.cells[0].rings?.five.pct).toBe(0);
  });

  it('names the auto account in full in the label', () => {
    const view = bubbleView(update([account('longname12ch', 81, 47, false, 58)]), NOW);
    expect(view.label).toBe(
      'AI usage. Next reset: longname12ch. 5 hour window: 81% used, resets in 47m. ' +
        '7 day window: 58% used, resets in 3d00h.'
    );
  });

  it('says ready in the label for an unused 5h window', () => {
    const view = bubbleView(update([account('acc1', 0, null, false, 100, 90)]), NOW);
    expect(view.label).toBe(
      'AI usage. Next reset: acc1. 5 hour window: 0% used, ready. 7 day window: 100% used, resets in 1h30m.'
    );
  });

  it('draws an unused 5h window as an empty ring, and a full 7d window as a full ring', () => {
    const view = bubbleView(update([account('acc1', 0, null, false, 100, 90)]), NOW);
    expect(view.cells[0].rings).toEqual({ five: { pct: 0, color: EMERALD }, seven: { pct: 100, color: RUBY } });
    expect(view.ears).toBe('both');
  });
});
