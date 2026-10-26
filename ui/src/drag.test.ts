import { describe, expect, it } from 'vitest';
import { type Gesture, IDLE, cancel, move, passedThreshold, press, release } from './drag';

describe('passedThreshold', () => {
  it('stays below the threshold under 4 px of total movement', () => {
    expect(passedThreshold(0, 0)).toBe(false);
    expect(passedThreshold(2, -1)).toBe(false);
  });

  it('passes at 4 px of total movement in any direction', () => {
    expect(passedThreshold(4, 0)).toBe(true);
    expect(passedThreshold(-2, 2)).toBe(true);
  });
});

// Runs a list of events from idle and returns every effect.
type Ev = ['down', number, boolean, number, number] | ['move', number, number, number] | ['up'] | ['cancel'];
function effects(events: Ev[]): (string | null)[] {
  let g: Gesture = IDLE;
  return events.map((e) => {
    const step =
      e[0] === 'down' ? press(g, e[1], e[2], e[3], e[4])
      : e[0] === 'move' ? move(g, e[1], e[2], e[3])
      : e[0] === 'up' ? release(g)
      : cancel(g);
    g = step.gesture;
    return step.effect;
  });
}

describe('click versus drag', () => {
  it('gives a click for a press and a release under the threshold', () => {
    expect(effects([['down', 0, true, 10, 10], ['move', 12, 11, 1], ['up']])).toEqual([null, null, 'click']);
  });

  it('starts one drag past the threshold and never gives a click', () => {
    expect(
      effects([['down', 0, true, 10, 10], ['move', 13, 11, 1], ['move', 30, 30, 1], ['up']])
    ).toEqual([null, 'start-drag', null, 'drag-end']);
  });

  it('ends a drag on the first move with no button down, when the release never came', () => {
    expect(effects([['down', 0, true, 0, 0], ['move', 4, 0, 1], ['move', 50, 50, 0], ['up']])).toEqual([
      null,
      'start-drag',
      'drag-end',
      null
    ]);
  });

  it('ends a missed drag on the next press, and that press can still click', () => {
    expect(effects([['down', 0, true, 0, 0], ['move', 9, 0, 1], ['down', 0, true, 5, 5], ['up']])).toEqual([
      null,
      'start-drag',
      'drag-end',
      'click'
    ]);
  });

  it('ignores a right or middle press and a press off a drag handle', () => {
    expect(effects([['down', 2, true, 0, 0], ['up']])).toEqual([null, null]);
    expect(effects([['down', 1, true, 0, 0], ['move', 20, 0, 4], ['up']])).toEqual([null, null, null]);
    expect(effects([['down', 0, false, 0, 0], ['move', 20, 0, 1], ['up']])).toEqual([null, null, null]);
  });

  it('drops a press on cancel but keeps a running drag', () => {
    expect(effects([['down', 0, true, 0, 0], ['cancel'], ['up']])).toEqual([null, null, null]);
    expect(effects([['down', 0, true, 0, 0], ['move', 5, 0, 1], ['cancel'], ['move', 5, 0, 0]])).toEqual([
      null,
      'start-drag',
      null,
      'drag-end'
    ]);
  });
});
