import { describe, expect, it } from 'vitest';
import { BLONDE, EMERALD, RUBY } from './colour';
import { layout } from './layout';
import { NOW, account } from './test-fixtures';
import { CONTROLS_HINT, bubbleView } from './model';

const update = (accounts: ReturnType<typeof account>[]) => ({ polledAt: NOW, accounts, error: null });

describe('bubbleView', () => {
  it('puts the auto account in the head and colours each ring by its own window', () => {
    const view = bubbleView(update([account('acc1', 42, 133, false, 63), account('acc2', 81, 47, false, 20)]), NOW);
    expect(view.cells.map((c) => c.rings)).toEqual([
      { five: { pct: 81, color: RUBY }, seven: { pct: 20, color: EMERALD } },
      { five: { pct: 42, color: EMERALD }, seven: { pct: 63, color: BLONDE } }
    ]);
    expect(view.cells.map((c) => c.id)).toEqual(['acc2', 'acc1']);
    expect(view.cells[0].r).toBeGreaterThan(view.cells[1].r);
    expect(view.cells[0].text).toEqual({
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

  it('highlights only the head, the auto account, and keeps the others in account order', () => {
    const view = bubbleView(update([account('acc1', 42, 133), account('acc2', 81, 47), account('acc3', 5, 90)]), NOW);
    expect(view.cells.map((c) => c.highlight)).toEqual([true, false, false]);
    expect(view.cells.map((c) => c.id)).toEqual(['acc2', 'acc1', 'acc3']);
  });

  it('rebalances when the auto account changes, with the same window size', () => {
    const accts = (soon: string) =>
      ['acc1', 'acc2', 'acc3', 'acc4'].map((id) => account(id, 10, id === soon ? 20 : 200));
    const a = bubbleView(update(accts('acc1')), NOW);
    const b = bubbleView(update(accts('acc4')), NOW);
    expect(a.cells.map((c) => c.id)).toEqual(['acc1', 'acc2', 'acc3', 'acc4']);
    expect(b.cells.map((c) => c.id)).toEqual(['acc4', 'acc1', 'acc2', 'acc3']);
    expect([b.layout.width, b.layout.height]).toEqual([a.layout.width, a.layout.height]);
  });

  it('makes the first account the head when no account is live', () => {
    const view = bubbleView(update([account('acc1', 42, 133, true), account('acc2', 81, 47, true)]), NOW);
    expect(view.cells.map((c) => c.id)).toEqual(['acc1', 'acc2']);
    expect(view.cells.some((c) => c.highlight)).toBe(false);
  });

  it('dims a stale cell, keeps it out of the highlight and out of the ear rule', () => {
    const view = bubbleView(update([account('acc1', 42, 133), account('acc2', 81, 47, true)]), NOW);
    expect(view.cells.map((c) => [c.id, c.dim, c.highlight])).toEqual([
      ['acc1', false, true],
      ['acc2', true, false]
    ]);
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
    expect(view.label).toBe('AI usage. No fresh data.' + CONTROLS_HINT);
  });

  it('shows 0% for a window whose reset passed after the poll', () => {
    const view = bubbleView(update([account('acc1', 90, -1)]), NOW);
    expect(view.cells[0].rings?.five.pct).toBe(0);
  });

  it('names the auto account in full in the label', () => {
    const view = bubbleView(update([account('longname12ch', 81, 47, false, 58)]), NOW);
    expect(view.label).toBe(
      'AI usage. Next reset: longname12ch. 5 hour window: 81% used, resets in 47m. ' +
        '7 day window: 58% used, resets in 3d00h.' + CONTROLS_HINT
    );
  });

  it('says ready in the label for an unused 5h window', () => {
    const view = bubbleView(update([account('acc1', 0, null, false, 100, 90)]), NOW);
    expect(view.label).toBe(
      'AI usage. Next reset: acc1. 5 hour window: 0% used, ready. 7 day window: 100% used, resets in 1h30m.' +
        CONTROLS_HINT
    );
  });

  it('draws an unused 5h window as an empty ring, and a full 7d window as a full ring', () => {
    const view = bubbleView(update([account('acc1', 0, null, false, 100, 90)]), NOW);
    expect(view.cells[0].rings).toEqual({ five: { pct: 0, color: EMERALD }, seven: { pct: 100, color: RUBY } });
    expect(view.ears).toBe('both');
  });

  it('puts a pinned account in the head and marks the pinned mode', () => {
    const accts = [account('acc1', 42, 133), account('acc2', 81, 47), account('acc3', 5, 90)];
    const auto = bubbleView(update(accts), NOW);
    const pinned = bubbleView(update(accts), NOW, 'acc3');
    expect(auto.mode).toBe('auto');
    expect(pinned.mode).toBe('pinned');
    expect(pinned.cells.map((c) => c.id)).toEqual(['acc3', 'acc1', 'acc2']);
    expect(pinned.cells.map((c) => c.highlight)).toEqual([true, false, false]);
    expect(pinned.label.startsWith('AI usage. Pinned: acc3.')).toBe(true);
    expect([pinned.layout.width, pinned.layout.height]).toEqual([auto.layout.width, auto.layout.height]);
  });

  it('falls back to auto when the pinned account is gone', () => {
    const view = bubbleView(update([account('acc1', 42, 133), account('acc2', 81, 47)]), NOW, 'acc9');
    expect(view.mode).toBe('auto');
    expect(view.cells[0].id).toBe('acc2');
  });
});
