// resilience/stop-chord-tool: the demo song is opened, the Piano roll is opened with F7, its TOOLS menu's chord progression tool is opened and "Start creating" is pressed (the ordinary
// way; no test hook stops the engine). The known defect #1386 stops the engine there. Expected: the stop is shown on the page (the engine's state is exited or error), and a reload
// brings the engine back to idle (the start completes). The stop's report (its reason and the last fle: line) is taken before the reload and kept in results.engineStop.
// Marked #1386 (expectFail): the scenario's ok is not required; the counts of the page's exceptions and failed requests must be 0.
import fs from 'node:fs';
import path from 'node:path';
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { dismissProblems, openProjectFile, sleep, waitProjectOpen, waitStable } from '../../lib/flow.mjs';
import { openMenu, persist, windowTable } from '../../lib/common.mjs';
import { absoluteRect } from '../../lib/dump.mjs';
import { allWindows, DEMO_PROJECT, press, stateName, waitUntil } from '../../lib/resilience.mjs';
import { installToastWatch } from '../../lib/detect.mjs';
import { stopOf } from '../../lib/stop.mjs';

const PIANO_ROLL = 'TEventEditForm';
const CHORD_TOOL = 'TPRChordToolForm';
const POPUP = 'TQuickPopupMenuWindow';
const TOOLS_DOWNS = 5;
const START_CREATING = { fx: 0.5, fy: 0.847 };

export default defineScenario({
  id: 'resilience/stop-chord-tool',
  covers: ['resilience/stop/state-shown', 'resilience/stop/reload-recovers'],
  displays: ['4k-dpr1'],
  expectFail: '#1386',
  async run({ guest, page, cdp, repo, log }) {
    await startIfStopped(guest, page, log);
    const notes = [];
    const reached = {};
    const demo = path.join(repo, 'lift', 'out-app', 'host', 'resources', DEMO_PROJECT);
    if (!fs.existsSync(demo)) return { ok: false, engineState: await stateName(guest), notes: [`no demo song at ${demo}`] };
    const first = await waitStable(guest, 180000);
    const empty = first.title ?? null;
    await openProjectFile(guest, cdp, demo, () => {});
    reached.demoOpen = (await waitProjectOpen(guest, empty, 120000)).opened;
    await dismissProblems(guest);
    await guest.focusDesktop();
    await press(guest, 'F7');
    reached.pianoRoll = await waitUntil(async () => allWindows(await windowTable(guest)).some((w) => w.cls === PIANO_ROLL && (w.name ?? '').startsWith('Piano roll')), 10000, 300);
    if (reached.pianoRoll) {
      await openMenu(guest, 'Tools');
      reached.toolsMenu = await waitUntil(async () => (await windowTable(guest)).top.some((w) => w.cls === POPUP), 3000, 200);
      for (let i = 0; i < TOOLS_DOWNS; i++) { await press(guest, 'ArrowDown'); await sleep(250); }
      await press(guest, 'Enter');
      reached.chordTool = await waitUntil(async () => allWindows(await windowTable(guest)).some((w) => w.cls === CHORD_TOOL), 8000, 300);
    }
    if (reached.chordTool) {
      const t = await windowTable(guest);
      const w = allWindows(t).find((x) => x.cls === CHORD_TOOL);
      const rect = absoluteRect(t, w.lid);
      await guest.click(rect.x + START_CREATING.fx * rect.w, rect.y + START_CREATING.fy * rect.h);
      reached.startCreating = true;
    }
    const stopped = await waitUntil(async () => { const s = await stateName(guest); return s === 'error' || s === 'exited'; }, 30000, 300);
    const st = await guest.state().catch(() => null);
    const shown = st ? (st.state === 'error' || st.state === 'exited') : false;
    const statusText = await guest.ev("(document.getElementById('status') || {}).textContent ?? null").catch(() => null);
    let engineStop = null;
    if (stopped) {
      const lines = await guest.ev('window.__app.engineLog()').catch(() => []);
      engineStop = stopOf(lines, st?.exit ?? null);
    }
    // the reload right after the stop, and the reload 45 s after it (a load soon after a stop found the stopped engine's files still held: see startIfStopped)
    let recoveredNow = false, recoveredLater = false;
    if (stopped) {
      const r = await persist(page, guest);
      await installToastWatch(page);
      recoveredNow = r.state === 'engine-idle';
      if (!recoveredNow) {
        await sleep(45000);
        const r2 = await persist(page, guest);
        await installToastWatch(page);
        recoveredLater = r2.state === 'engine-idle';
      }
    }
    const recovered = recoveredNow || recoveredLater;
    const reason = engineStop ? `${engineStop.exit?.reason ?? st?.state} ${engineStop.exit?.code ?? ''}: ${String(engineStop.message ?? '').slice(0, 160)}` : 'no stop';
    notes.push(`reached ${JSON.stringify(reached)}; stopped ${stopped}, shown as ${st?.state ?? 'unreadable'} (page status "${String(statusText ?? '').slice(0, 80)}"), reload at once ${recoveredNow ? 'idle' : 'not idle'}, reload 45 s later ${stopped && !recoveredNow ? (recoveredLater ? 'idle' : 'not idle') : 'not needed'}; ${reason}`);
    return {
      ok: stopped && shown && recovered, engineState: await stateName(guest), notes,
      results: { reached, stopped, stateShown: st?.state ?? null, exit: st?.exit ?? null, statusText, engineStop, recoveredNow, recoveredLater, recovered },
    };
  },
});
