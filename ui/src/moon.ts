// The rabbit in the moon. Each 7d row of the card shows a moon that fills as the reset gets
// closer. A port of the moon maths in docs/design/mockup/Card.dc.html.

import { SEVEN_DAY_MINUTES } from './usage';

/** The radius and the centre of the moon in its 12 px box. */
const MOON_R = 5;
const MOON_C = 6;

const TOP = MOON_C + ' ' + (MOON_C - MOON_R);
const BOTTOM = MOON_C + ' ' + (MOON_C + MOON_R);

/**
 * Returns the fraction of the 7d window that has passed. The moon is new (0) just after a
 * reset and full (1) at the next reset.
 */
export function moonPhase(minsLeft: number): number {
  return Math.max(0, Math.min(1, 1 - minsLeft / SEVEN_DAY_MINUTES));
}

/** The whole moon disc, drawn as two half arcs. */
export const MOON_DISC =
  'M ' + TOP + ' A ' + MOON_R + ' ' + MOON_R + ' 0 0 1 ' + BOTTOM +
  ' A ' + MOON_R + ' ' + MOON_R + ' 0 0 1 ' + TOP + ' Z';

/**
 * Returns the lit part of a waxing moon at phase `f`. The right half arc is the limb. The
 * terminator is a half ellipse with x radius r * |1 - 2f|. It bulges right (a crescent)
 * under half and bulges left (gibbous) over half. A new moon has no lit part.
 */
export function moonLit(f: number): string {
  if (f <= 0) return '';
  const rx = (MOON_R * Math.abs(1 - 2 * f)).toFixed(2);
  const sweep = f < 0.5 ? 0 : 1;
  return 'M ' + TOP + ' A ' + MOON_R + ' ' + MOON_R + ' 0 0 1 ' + BOTTOM +
    ' A ' + rx + ' ' + MOON_R + ' 0 0 ' + sweep + ' ' + TOP + ' Z';
}

export interface MoonView {
  lit: string;
  label: string;
}

/** Returns the moon of a 7d window with `minsLeft` to its reset. `left` is the countdown text. */
export function moonView(minsLeft: number, left: string): MoonView {
  const f = moonPhase(minsLeft);
  return { lit: moonLit(f), label: 'Moon ' + Math.round(f * 100) + '% full, 7d reset in ' + left };
}

/** The moon of a 7d window with no countdown. */
export const NEW_MOON: MoonView = { lit: '', label: 'New moon, 7d window ready' };
