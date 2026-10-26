import { describe, expect, it } from 'vitest';
import { NO_DATA } from './centre';
import { COL_X, TEXT_LIMIT, cellItems, fitCell, itemHalfWidth, itemNorm } from './fit';
import { CELL_APOTHEM, FIVE_RING } from './geometry';

const text = (name: string, fiveCd = '1h12m', sevenCd = '2d04h', fivePct = '33%', sevenPct = '33%') => ({
  name,
  five: { pct: fivePct, countdown: fiveCd },
  seven: { pct: sevenPct, countdown: sevenCd }
});

describe('cellItems', () => {
  it('puts the 5h items on the left, the 7d items on the right and the name in the middle', () => {
    const items = cellItems(text('acc1'));
    const at = (t: string) => items.find((i) => i.text === t);
    expect(items.filter((i) => i.x < 0).map((i) => i.text)).toEqual(['33%', '1h12m']);
    expect(items.filter((i) => i.x > 0).map((i) => i.text)).toEqual(['33%', '2d04h']);
    expect(at('acc1')).toMatchObject({ role: 'name', x: 0 });
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

  it('keeps a 12 character name whole with the widest normal figures', () => {
    expect(fitCell(text('longname12ch', '4h59m', '23h59m', '100%', '100%')).name).toBe('longname12ch');
  });

  it('cuts a longer name with an ellipsis and keeps the figures whole', () => {
    const fit = fitCell(text('averylongname20chars'));
    expect(fit.name).toMatch(/^averylong\w*…$/);
    expect(fit.items.map((i) => i.text)).toContain('2d04h');
  });

  it('never puts an item outside the 5h ring, and never lets two columns touch', () => {
    const names = ['a', 'acc3', 'kiriket', 'longname12ch', 'averylongname20chars'];
    const fives = ['ready', '33m', '4h59m', '--'];
    const sevens = ['ready', '33m', '23h59m', '6d23h', '--'];
    const pcts = ['0%', '33%', '100%', '--'];
    for (const name of names) {
      for (const f of fives) {
        for (const s of sevens) {
          for (const p of pcts) {
            const fit = fitCell(text(name, f, s, p, p));
            fit.items.forEach((i) => expect(itemNorm(i)).toBeLessThanOrEqual(TEXT_LIMIT));
            for (const role of ['pct', 'countdown']) {
              const [l, r] = fit.items.filter((i) => i.role === role);
              expect(r.x - itemHalfWidth(r) - (l.x + itemHalfWidth(l))).toBeGreaterThanOrEqual(2);
            }
          }
        }
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
