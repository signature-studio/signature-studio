// resilience/demo-open-replace-50: the demo song DEDZII - NIGHT TERROR is opened, then File > New (Ctrl+N) puts the empty project back, 50 times. Expected: each time the song opens
// (the title changes, the engine idle), and each time New brings the empty project back (the title of the first empty project); no stop; the memory is read at the start and the end.
import fs from 'node:fs';
import path from 'node:path';
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { resetProject } from '../../lib/common.mjs';
import { dismissProblems, openProjectFile, sleep, waitProjectOpen, WINDOW } from '../../lib/flow.mjs';
import { DEMO_PROJECT, memoryBytes, stateName } from '../../lib/resilience.mjs';

/** the Open box (the file manager window) is gone: its Open was answered (true), or it is still up at the limit (false) */
async function boxAnswered(guest, ms) {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    if (!(await guest.ev(WINDOW))) return true;
    await sleep(300);
  }
  return false;
}

export default defineScenario({
  id: 'resilience/demo-open-replace-50',
  covers: ['resilience/project/demo-open-replace'],
  displays: ['4k-dpr1'],
  expectFail: 'new', // run of 2026-11-28: the 2nd Ctrl+O's file box is not seen in 40 s (the page settles dialog 2 with the demo file: history answered); a candidate for PM triage, not confirmed
  async run({ guest, cdp, repo, page, log }) {
    await startIfStopped(guest, page, log);
    const demo = path.join(repo, 'lift', 'out-app', 'host', 'resources', DEMO_PROJECT);
    if (!fs.existsSync(demo)) return { ok: false, engineState: await stateName(guest), notes: [`no demo song at ${demo}`] };
    const empty = (await guest.state())?.title ?? null;
    const mem0 = await memoryBytes(guest);
    let opened = 0, replaced = 0, answers = 0;
    const fails = [];
    for (let i = 1; i <= 50; i++) {
      await openProjectFile(guest, cdp, demo, () => {});
      if (await boxAnswered(guest, 10000)) answers++; else fails.push({ cycle: i, step: 'answer', state: (await stateName(guest)) });
      const end = await waitProjectOpen(guest, empty, 120000);
      if (end.opened) opened++; else fails.push({ cycle: i, step: 'open', state: end.state });
      await dismissProblems(guest);
      await resetProject(guest);
      const s = await guest.state();
      if (s && s.title === empty && s.state === 'engine-idle') replaced++; else fails.push({ cycle: i, step: 'new', state: s?.state ?? null });
      if (s && (s.state === 'error' || s.state === 'exited')) break;
    }
    const mem1 = await memoryBytes(guest);
    const state = await stateName(guest);
    return {
      ok: opened === 50 && replaced === 50 && answers === 50 && state === 'engine-idle', engineState: state,
      notes: [`demo answered (Open) ${answers} of 50, opened ${opened} of 50 (the title changed), replaced by New ${replaced}; fails ${JSON.stringify(fails.slice(0, 5))}; memory ${mem0} -> ${mem1}`],
      results: { cycles: 50, answers, opened, replaced, fails: fails.length, memBytes: { start: mem0, end: mem1 } },
    };
  },
});
