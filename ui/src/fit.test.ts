import { describe, expect, it } from 'vitest';
import { NO_DATA } from './centre';
import { COL_X, MARK_GAP, MARK_SIZE, NAME_SIZES, ROW_SIZE, TEXT_LIMIT, cellItems, fitCell, itemHalfWidth, itemNorm } from './fit';
import { APOTHEM, CELL_APOTHEM, FIVE_RING } from './geometry';
import type { ProviderName } from './usage';

const text = (
  name: string,
  fiveCd = '1h12m',
  sevenCd = '2d04h',
  fivePct = '33%',
  sevenPct = '33%',
  provider: ProviderName | null = 'claude'
) => ({
  provider,
  name,
  noData: false,
  five: { pct: fivePct, countdown: fiveCd },
  seven: { pct: sevenPct, countdown: sevenCd }
});

describe('cellItems', () => {
  it('puts the 5h items on the left, the 7d items on the right and the name in the middle', () => {
    const items = cellItems(text('acc1'));
    const at = (t: string) => items.find((i) => i.text === t);
    expect(items.filter((i) => i.x < 0).map((i) => i.text)).toEqual(['33%', '1h12m']);
    expect(items.filter((i) => i.x > 0 && i.role !== 'name').map((i) => i.text)).toEqual(['33%', '2d04h']);
    expect(at('acc1')).toMatchObject({ role: 'name', x: (MARK_SIZE + MARK_GAP) / 2 });
    const bare = cellItems(text('acc1', undefined, undefined, undefined, undefined, null));
    expect(bare.find((i) => i.role === 'name')).toMatchObject({ x: 0 });
  });

  it('lines each percent up with its countdown on fixed columns', () => {
    const items = cellItems(text('acc1'));
    expect(new Set(items.filter((i) => i.role !== 'name').map((i) => Math.abs(i.x)))).toEqual(new Set([COL_X]));
  });

  it('stacks the rows percent, name, countdown on the cell centre', () => {
    const items = cellItems(text('acc1'));
    const y = (role: string) => items.find((i) => i.role === role)?.y ?? NaN;
    expect(y('pct')).toBeLessThan(y('name'));
    expect(y('name')).toBeLessThan(y('countdown'));
    expect(Math.abs(y('pct') + y('countdown'))).toBeLessThan(2);
  });
});

describe('fitCell', () => {
  it('keeps a short name whole', () => {
    expect(fitCell(text('acc1')).name).toBe('acc1');
  });

  it('keeps a name of up to 12 characters whole after the mark, with a smaller font when needed', () => {
    const size = (name: string) => {
      const fit = fitCell(text(name, '4h59m', '23h59m', '100%', '100%'));
      return [fit.name, fit.items.find((i) => i.role === 'name')?.size];
    };
    expect(size('acc1')).toEqual(['acc1', ROW_SIZE.name]);
    expect(size('longname10')).toEqual(['longname10', ROW_SIZE.name]);
    expect(size('longname11c')).toEqual(['longname11c', NAME_SIZES[1]]);
    expect(size('longname12ch')).toEqual(['longname12ch', NAME_SIZES[2]]);
    expect(fitCell(text('longname13chr')).name).toBe('longname13c…');
  });

  it('steps the name font down by at most 1 px', () => {
    expect(NAME_SIZES[0]).toBe(ROW_SIZE.name);
    expect(ROW_SIZE.name - Math.min(...NAME_SIZES)).toBeLessThanOrEqual(1);
  });

  it('keeps a 12 character name whole with no mark', () => {
    const bare = text('longname12ch', '4h59m', '23h59m', '100%', '100%', null);
    expect(fitCell(bare).name).toBe('longname12ch');
    expect(fitCell(bare).mark).toBeNull();
  });

  it('puts the mark before the name and centres the two on the cell', () => {
    const fit = fitCell(text('acc1'));
    const name = fit.items.find((i) => i.role === 'name');
    if (!fit.mark || !name) throw new Error('The fit has no mark or no name.');
    expect(fit.mark.y).toBe(name.y);
    const left = fit.mark.x - MARK_SIZE / 2;
    const right = name.x + itemHalfWidth(name);
    expect(left + right).toBeCloseTo(0, 6);
    expect(name.x - itemHalfWidth(name) - (fit.mark.x + MARK_SIZE / 2)).toBeCloseTo(MARK_GAP, 6);
  });

  it('cuts a longer name with an ellipsis and keeps the figures whole', () => {
    const fit = fitCell(text('averylongname20chars'));
    expect(fit.name).toMatch(/^averylong\w*…$/);
    expect(fit.items.map((i) => i.text)).toContain('2d04h');
  });

  it('never puts an item or the mark outside the 5h ring, and never lets two columns touch', () => {
    const names = ['a', 'acc3', 'kiriket', 'longname11c', 'longname12ch', 'averylongname20chars'];
    const fives = ['ready', '33m', '4h59m', '--'];
    const sevens = ['ready', '33m', '23h59m', '6d23h', '--'];
    const pcts = ['0%', '33%', '100%', '--'];
    const providers: (ProviderName | null)[] = ['claude', 'codex', null];
    const cases = names.flatMap((n) =>
      fives.flatMap((f) => sevens.flatMap((s) => pcts.flatMap((p) => providers.map((v) => text(n, f, s, p, p, v)))))
    );
    for (const c of cases) {
      const fit = fitCell(c);
      fit.items.forEach((i) => expect(itemNorm(i)).toBeLessThanOrEqual(TEXT_LIMIT));
      if (fit.mark) {
        const dy = Math.abs(fit.mark.y) + MARK_SIZE / 2;
        const dx = Math.abs(fit.mark.x) + MARK_SIZE / 2;
        expect(Math.max(dy, dx * APOTHEM + dy / 2)).toBeLessThanOrEqual(TEXT_LIMIT);
      }
      for (const role of ['pct', 'countdown']) {
        const [l, r] = fit.items.filter((i) => i.role === role);
        expect(r.x - itemHalfWidth(r) - (l.x + itemHalfWidth(l))).toBeGreaterThanOrEqual(2);
      }
    }
  });

  it('keeps the text limit inside the 5h ring and the cell', () => {
    expect(TEXT_LIMIT).toBeLessThan(FIVE_RING.inner);
    expect(TEXT_LIMIT).toBeLessThan(CELL_APOTHEM);
  });

  it('fits the no data text whole', () => {
    expect(fitCell(NO_DATA).name).toBe('no data');
  });
});
