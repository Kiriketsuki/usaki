import { describe, expect, it } from 'vitest';
import { NO_DATA } from './centre';
import { TEXT_LIMIT, cellLines, fitCell, lineNorms } from './fit';
import { CELL_APOTHEM, SEVEN_RING } from './geometry';

const text = (name: string, countdown = '2h 13m', figures = '81/58%') => ({ countdown, name, figures });

describe('fitCell', () => {
  it('keeps a short name whole and the block centred', () => {
    expect(fitCell(text('acc1'))).toEqual({ name: 'acc1', offset: 0 });
  });

  it('keeps a 12 character name whole', () => {
    expect(fitCell(text('longname12ch')).name).toBe('longname12ch');
  });

  it('cuts a longer name with an ellipsis and keeps the other lines whole', () => {
    const fit = fitCell(text('averylongname20chars'));
    expect(fit.name).toMatch(/^averylong\w*…$/);
    expect(fit.name.length).toBeLessThan(20);
  });

  it('never puts a line outside the 7d ring', () => {
    const names = ['a', 'acc3', 'kiriket', 'longname12ch', 'averylongname20chars'];
    const countdowns = ['now', '47m', '4h 59m', '--'];
    const figs = ['0/0%', '81/58%', '100/100%', '--/--%'];
    for (const name of names) {
      for (const countdown of countdowns) {
        for (const figures of figs) {
          const t = text(name, countdown, figures);
          const fit = fitCell(t);
          const norms = lineNorms(cellLines({ ...t, name: fit.name }), fit.offset);
          expect(Math.max(...norms)).toBeLessThanOrEqual(TEXT_LIMIT);
        }
      }
    }
  });

  it('keeps the text limit inside the 7d ring and the cell', () => {
    expect(TEXT_LIMIT).toBeLessThan(SEVEN_RING.inner);
    expect(TEXT_LIMIT).toBeLessThan(CELL_APOTHEM);
  });

  it('fits the no data text whole', () => {
    expect(fitCell(NO_DATA).name).toBe('no data');
  });
});

describe('lineNorms', () => {
  it('grows with the line length and with the distance from the centre', () => {
    const short = lineNorms(cellLines(text('a')));
    const long = lineNorms(cellLines(text('longname12ch')));
    expect(long[1]).toBeGreaterThan(short[1]);
    expect(lineNorms(cellLines(text('a')), -4)[2]).toBeLessThan(short[2]);
  });
});
