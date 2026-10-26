// The debug script: a comma-separated list of mouse actions, such as `right,right,middle,open`.
// Only a debug build reads it, from the USAKI_DEBUG_UI environment variable.

export type DebugAction = 'right' | 'middle' | 'open' | 'close';

const ACTIONS: readonly DebugAction[] = ['right', 'middle', 'open', 'close'];

/** Returns the actions of `script`. It skips a word that is not an action. */
export function parseScript(script: string | null | undefined): DebugAction[] {
  return (script ?? '')
    .split(',')
    .map((w) => w.trim())
    .filter((w): w is DebugAction => (ACTIONS as readonly string[]).includes(w));
}
