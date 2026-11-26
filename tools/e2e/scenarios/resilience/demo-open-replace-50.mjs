// resilience/demo-open-replace-50: the demo song DEDZII - NIGHT TERROR is opened, then File > New (Ctrl+N) puts the empty project back, 50 times. Expected: each time the song opens
// (the title changes, the engine idle), and each time New brings the empty project back (the title of the first empty project); no stop; the memory is read at the start and the end.
import fs from 'node:fs';
import path from 'node:path';
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { resetProject } from '../../lib/common.mjs';
import { dismissProblems, openProjectFile, waitProjectOpen } from '../../lib/flow.mjs';
import { DEMO_PROJECT, memoryBytes, stateName } from '../../lib/resilience.mjs';

export default defineScenario({
  id: 'resilience/demo-open-replace-50',
  covers: ['resilience/project/demo-open-replace'],
  displays: ['4k-dpr1'],
  async run({ guest, cdp, repo, page, log }) {
    await startIfStopped(guest, page, log);
    const demo = path.join(repo, 'lift', 'out-app', 'host', 'resources', DEMO_PROJECT);
    if (!fs.existsSync(demo)) return { ok: false, engineState: await stateName(guest), notes: [`no demo song at ${demo}`] };
    const empty = (await guest.state())?.title ?? null;
    const mem0 = await memoryBytes(guest);
    let opened = 0, replaced = 0;
    const fails = [];
    for (let i = 1; i <= 50; i++) {
      await openProjectFile(guest, cdp, demo, () => {});
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
      ok: opened === 50 && replaced === 50 && state === 'engine-idle', engineState: state,
      notes: [`demo opened ${opened} of 50, replaced by New ${replaced}; fails ${JSON.stringify(fails.slice(0, 5))}; memory ${mem0} -> ${mem1}`],
      results: { cycles: 50, opened, replaced, fails: fails.length, memBytes: { start: mem0, end: mem1 } },
    };
  },
});
