// resilience/plugin-list-cycle-20: the plugin list is opened with F8 and closed with Escape, 20 times. Expected: each time the window is in the window table after the open and gone after
// the close; no stop; the engine's memory does not grow (the ratio at most 1.10).
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { cycleWindow, memoryRatio, MEM_RATIO, stateName } from '../../lib/resilience.mjs';

const PLUG_LIST = 'TPlugListForm';

export default defineScenario({
  id: 'resilience/plugin-list-cycle-20',
  covers: ['resilience/window/plugin-list-cycle'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    await guest.focusDesktop();
    const r = await cycleWindow(guest, { cls: PLUG_LIST, open: 'F8', close: ['Escape'] }, 20);
    const ratio = memoryRatio(r.memory);
    const state = await stateName(guest);
    const ok = r.opened === 20 && r.closed === 20 && r.fails.length === 0 && ratio !== null && ratio <= MEM_RATIO && state === 'engine-idle';
    return {
      ok, engineState: state,
      notes: [`plugin list opened ${r.opened} of ${r.cycles}, closed ${r.closed}; memory ratio ${ratio} (limit ${MEM_RATIO}); fails ${JSON.stringify(r.fails.slice(0, 5))}`],
      results: { cycles: r.cycles, opened: r.opened, closed: r.closed, fails: r.fails.length, memory: r.memory, memoryRatio: ratio },
    };
  },
});
