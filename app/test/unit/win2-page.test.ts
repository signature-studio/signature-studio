import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

// Decision 0222 (win2): the page shows what the engine's windows are, it does not decide what they do. One text check of the sources (a DOM is not needed to see it): when the engine has returned (the application ended and the
// runtime destroyed the windows it left: lift/runtime/fle_win.c fle_win_process_ended), the Worker draws the desktop once more (the picture is made from the runtime's window table, decision 0223), so that the desktop shows
// what the system would show; an engine that stopped or trapped keeps its last picture (its windows are not destroyed).
const APP = path.resolve(import.meta.dirname, '../..');
const read = (f: string): string => fs.readFileSync(path.join(APP, f), 'utf8');

describe('decision 0222 with the desktop compositor (decision 0223): the desktop after the engine returned', () => {
  it('the Worker draws the desktop once more when run() has returned, from the runtime window table (the page has no window list to read); a stopped or trapped engine is not given a different treatment', () => {
    const w = read('src/engine/worker.ts');
    const i = w.indexOf('const result = await f.run();');
    expect(i).toBeGreaterThan(0);
    const after = w.slice(i, i + 600);
    expect(after).toContain('deskTick()');
    expect(after.indexOf('deskTick()')).toBeLessThan(after.indexOf("post({ t: 'exit'"));
    expect(after).toMatch(/\n  try \{ deskTick\(\); \}/); // at the level of the function: not inside a condition on how the engine ended
    expect(w).not.toContain('pollAppView');
  });
});
