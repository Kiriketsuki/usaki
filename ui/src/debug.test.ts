import { describe, expect, it } from 'vitest';
import { parseScript } from './debug';

describe('parseScript', () => {
  it('reads the actions in order and skips other words', () => {
    expect(parseScript(' right,right , middle,open,jump,close')).toEqual(['right', 'right', 'middle', 'open', 'close']);
  });

  it('gives no action for no script', () => {
    expect(parseScript(null)).toEqual([]);
    expect(parseScript('')).toEqual([]);
  });
});
