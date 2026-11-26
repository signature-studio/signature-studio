// resilience/reload-20: the page is reloaded 20 times; each time the start runs again to the idle engine and the default project, and the window table is read after it.
// Expected: every reload reaches engine-idle with the project open and the main form in the table; no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { MAIN_FORM, persist, windowTable } from '../../lib/common.mjs';
import { installToastWatch } from '../../lib/detect.mjs';
import { stateName } from '../../lib/resilience.mjs';

export default defineScenario({
  id: 'resilience/reload-20',
  covers: ['resilience/reload/start-completes'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    const rows = [];
    for (let i = 1; i <= 20; i++) {
      const r = await persist(page, guest);
      await installToastWatch(page);
      const main = (await windowTable(guest).catch(() => ({ top: [] }))).top.some((w) => w.cls === MAIN_FORM);
      rows.push({ i, state: r.state, opened: r.opened, main });
    }
    const idle = rows.filter((r) => r.state === 'engine-idle' && r.opened && r.main).length;
    const state = await stateName(guest);
    return {
      ok: idle === 20 && state === 'engine-idle', engineState: state,
      notes: [`20 reloads: engine-idle with the project open and the main form ${idle} times; states ${JSON.stringify([...new Set(rows.map((r) => r.state))])}`],
      results: { reloads: 20, startsCompleted: idle, states: rows.map((r) => r.state) },
    };
  },
});
