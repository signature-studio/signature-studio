// resilience/reload-then-open-project: the page is reloaded (the start completes), then the demo song DEDZII - NIGHT TERROR is opened from the File menu's open box (Ctrl+O).
// Expected: the start completes after the reload, and the song opens (the title changes, the engine idle); no stop.
import fs from 'node:fs';
import path from 'node:path';
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { persist } from '../../lib/common.mjs';
import { installToastWatch } from '../../lib/detect.mjs';
import { dismissProblems, openProjectFile, waitProjectOpen } from '../../lib/flow.mjs';
import { DEMO_PROJECT, stateName } from '../../lib/resilience.mjs';

export default defineScenario({
  id: 'resilience/reload-then-open-project',
  covers: ['resilience/reload/then-open-project'],
  displays: ['4k-dpr1'],
  async run({ guest, page, cdp, repo, log }) {
    await startIfStopped(guest, page, log);
    const demo = path.join(repo, 'lift', 'out-app', 'host', 'resources', DEMO_PROJECT);
    if (!fs.existsSync(demo)) return { ok: false, engineState: await stateName(guest), notes: [`no demo song at ${demo}`] };
    const r = await persist(page, guest);
    await installToastWatch(page);
    if (r.state !== 'engine-idle') return { ok: false, engineState: r.state, notes: [`after the reload: state ${r.state}`] };
    const before = (await guest.state())?.title ?? null;
    await openProjectFile(guest, cdp, demo, () => {});
    const end = await waitProjectOpen(guest, before, 120000);
    await dismissProblems(guest);
    const state = await stateName(guest);
    return {
      ok: end.opened === true && state === 'engine-idle', engineState: state,
      notes: [`after the reload (${r.state}, project ${r.opened}): the demo song ${end.opened ? 'opened' : 'did not open'} (title "${end.title}", state ${end.state}) in ${end.waitedMs} ms`],
      results: { reloadState: r.state, opened: end.opened, waitedMs: end.waitedMs },
    };
  },
});
