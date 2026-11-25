// (c) file-open-demo: Welcome closed with Escape (the default project opens), then File > Open (its shortcut Ctrl+O, the engine's Open) and the installer's
// demo song "DEDZII - NIGHT TERROR" (Data/Demo projects/Demo songs of the bundle's installer resources) given to the open box. The engine does not stop and the project opens.
// Issue #1421 (the expected failure): the engine stops with exit code 134 about 5 s after the open; the stop reason is in the scenario's JSON (engineStop).
import path from 'node:path';
import { dismissProblems, noThanks, openProjectFile, sleep, waitProjectOpen, waitStable } from '../../lib/flow.mjs';

export const DEMO = path.join('Data', 'Demo projects', 'Demo songs', 'DEDZII - NIGHT TERROR.flp');

export default {
  name: 'file-open-demo',
  expectFail: '#1421',
  async run({ guest, cdp, repo, log }) {
    const notes = [];
    const first = await waitStable(guest, 180000);
    notes.push(`first state ${first.state} after ${first.waitedMs} ms`);
    if (first.state !== 'engine-idle') return { ok: false, engineState: first.state, notes };
    await noThanks(guest);
    await guest.escape();
    const def = await waitProjectOpen(guest, first.title, 120000);
    notes.push(`default project ${def.opened ? 'open' : 'not open'}: title "${def.title}", state ${def.state}`);
    if (!def.opened) return { ok: false, engineState: def.state, notes };
    if (await dismissProblems(guest)) notes.push('the "Problems loading the project" box of the default project was closed');
    const demoPath = path.join(repo, 'lift', 'out-app', 'host', 'resources', DEMO);
    await openProjectFile(guest, cdp, demoPath, (m) => { notes.push(m); log(m); });
    const t0 = Date.now();
    let last = null;
    let problemsDuringOpen = false;
    while (Date.now() - t0 < 240000) {
      last = await guest.state().catch(() => null);
      if (last && (last.state === 'error' || last.state === 'exited')) break;
      if (last && last.state === 'engine-idle' && last.title !== def.title && Date.now() - t0 > 3000) break;
      if (await dismissProblems(guest)) problemsDuringOpen = true;
      await sleep(500);
    }
    await sleep(1500);
    const end = (await guest.state().catch(() => null)) ?? { state: 'unreadable', title: null };
    const opened = end.state === 'engine-idle' && end.title !== def.title;
    notes.push(`demo open ${opened} after ${Date.now() - t0} ms: title "${end.title}", state ${end.state}${problemsDuringOpen ? ', a Problems box came' : ''}`);
    return { ok: opened, engineState: end.state, notes };
  },
};
