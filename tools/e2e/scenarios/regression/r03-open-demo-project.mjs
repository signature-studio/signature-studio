// R03 (#1408) open-demo-project. Report (L-057): a demo project (the installer's Demo songs) is opened with File > Open. Expectation: the engine does not stop,
// the window's title changes (the project is open), and the engine stays engine-idle for 20 s after that (read every 500 ms).
import path from 'node:path';
import { dismissProblems, openProjectFile, sleep, waitProjectOpen } from '../../lib/flow.mjs';
import { engineLines, startDefault, result } from './common.mjs';

const SCENARIO = {
  name: 'open-demo-project',
  report: 'R03',
  expect: 'File > Open of the installer demo song (DEDZII - NIGHT TERROR.flp): the title changes; engine-idle at every read for 20 s after it opens',
  // #1421 (the DEDZII load stopped the engine) is closed by its fix on main; a closed issue is not a mark
  expectFail: null,
};
export const DEMO = path.join('Data', 'Demo projects', 'Demo songs', 'DEDZII - NIGHT TERROR.flp');
export const HOLD_MS = 20000;

export default {
  ...SCENARIO,
  async run({ guest, cdp, repo, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    const demo = path.join(repo, 'lift', 'out-app', 'host', 'resources', DEMO);
    await openProjectFile(guest, cdp, demo, log);
    const opened = await waitProjectOpen(guest, start.title, 240000);
    if (!opened.opened) return result(false, opened.state, [...start.notes, `the demo did not open: state ${opened.state}, title "${opened.title}"`]);
    const notes = [`opened "${opened.title}" after ${opened.waitedMs} ms`];
    const seen = {};
    const t0 = Date.now();
    let stopped = false;
    while (Date.now() - t0 < HOLD_MS) {
      await dismissProblems(guest);
      const s = await guest.state().catch(() => null);
      const st = s?.state ?? 'unreadable';
      seen[st] = (seen[st] ?? 0) + 1;
      if (st !== 'engine-idle') { stopped = true; break; }
      await sleep(500);
    }
    notes.push(`states over ${Date.now() - t0} ms after the open: ${JSON.stringify(seen)}`);
    log(notes.at(-1));
    if (stopped) notes.push(`the engine log, the last lines: ${JSON.stringify((await engineLines(guest)).slice(-12).map((l) => String(l).slice(0, 200)))}`);
    return result(opened.opened && !stopped, (await guest.state())?.state ?? null, [...start.notes, ...notes], { seen });
  },
};
