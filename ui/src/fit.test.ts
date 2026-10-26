import { describe, expect, it } from 'vitest';
import { NO_DATA } from './centre';
import { TEXT_MARGIN, fitText, lineNorms, linesOf } from './fit';
import { APOTHEM, OUTLINE_R, ringsInner } from './geometry';

const text = (name: string, centre = '2h 13m', figures = '81/58%') => ({
  top: 'next 5h',
  centre,
  name,
  figures
});

const limitFor = (count: number) => ringsInner(count) - TEXT_MARGIN;

describe('fitText', () => {
  it('keeps the mockup sizes and the full line for a short name', () => {
    expect(fitText(text('acc1'), limitFor(1))).toEqual({
      sizes: { top: 9, centre: 17, sub: 10 },
      sub: 'acc1 81/58%',
      offset: 0
    });
  });

  it('shows a 12 character name in full with 1 account, one size step down', () => {
    const fit = fitText(text('longname12ch'), limitFor(1));
    expect(fit.sub).toBe('longname12ch 81/58%');
    expect(fit.sizes.sub).toBe(9);
  });

  it('cuts a long name with an ellipsis and keeps the figures whole', () => {
    for (const count of [2, 3, 4, 5]) {
      const fit = fitText(text('longname12ch'), limitFor(count));
      expect(fit.sub).toMatch(/^longna\w*\u2026 81\/58%$/);
      expect(fit.sub.endsWith(' 81/58%')).toBe(true);
    }
  });

  it('keeps at least 6 name characters with up to 5 accounts', () => {
    for (const count of [1, 2, 3, 4, 5]) {
      const name = fitText(text('longname12ch'), limitFor(count)).sub.split(' ')[0];
      expect(name.replace('\u2026', '').length).toBeGreaterThanOrEqual(6);
    }
  });

  it('keeps the size step for one count when the name changes', () => {
    for (const count of [1, 2, 3, 4, 5]) {
      const a = fitText(text('acc1'), limitFor(count)).sizes;
      const b = fitText(text('longname12ch'), limitFor(count)).sizes;
      expect([a.top, a.centre]).toEqual([b.top, b.centre]);
    }
  });

  it('never puts a line outside the innermost ring or the hex', () => {
    const outline = OUTLINE_R * APOTHEM;
    const names = ['a', 'acc3', 'kiriket', 'longname12ch'];
    const centres = ['now', '47m', '4h 59m', '--'];
    for (const count of [0, 1, 2, 3, 4, 5]) {
      const limit = limitFor(count);
      for (const name of names) {
        for (const centre of centres) {
          const fit = fitText(text(name, centre), limit);
          const norms = lineNorms(linesOf('next 5h', centre, fit.sub, fit.sizes), fit.offset);
          expect(Math.max(...norms)).toBeLessThanOrEqual(Math.min(limit, outline));
          expect(Math.abs(fit.offset)).toBeLessThanOrEqual(10);
        }
      }
    }
  });

  it('fits the no data text without a cut', () => {
    expect(fitText(NO_DATA, limitFor(5)).sub).toBe('offline');
  });
});

describe('lineNorms', () => {
  it('grows with the line length and with the distance from the centre', () => {
    const lines = (sub: string) => linesOf('next 5h', '47m', sub, { top: 9, centre: 17, sub: 10 });
    const short = lineNorms(lines('a 1/1%'));
    const long = lineNorms(lines('longname12ch 81/58%'));
    expect(long[2]).toBeGreaterThan(short[2]);
    expect(lineNorms(lines('a 1/1%'), -6)[2]).toBeLessThan(short[2]);
  });
});
