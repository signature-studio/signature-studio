// resilience/channel-rack-cycle-100: the Channel rack window is opened with F6 and closed again (F6, or Escape when F6 does not close it), 100 times. Expected: each time the window
// is in the window table after the open and gone after the close; no stop; the engine's memory does not grow (the last report over the first: at most 1.10).
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { cycleWindow, memoryRatio, MEM_RATIO, stateName } from '../../lib/resilience.mjs';

const CHANNEL_RACK = 'TStepSeqForm';

export default defineScenario({
  id: 'resilience/channel-rack-cycle-100',
  covers: ['resilience/window/channel-rack-cycle'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    await guest.focusDesktop();
    const r = await cycleWindow(guest, { cls: CHANNEL_RACK, open: 'F6', close: ['F6', 'Escape'] }, 100);
    const ratio = memoryRatio(r.memory);
    const state = await stateName(guest);
    const ok = r.opened === 100 && r.closed === 100 && r.fails.length === 0 && ratio !== null && ratio <= MEM_RATIO && state === 'engine-idle';
    return {
      ok, engineState: state,
      notes: [`Channel rack opened ${r.opened} of ${r.cycles}, closed ${r.closed}; memory ratio ${ratio} (limit ${MEM_RATIO}); fails ${JSON.stringify(r.fails.slice(0, 5))}`],
      results: { cycles: r.cycles, opened: r.opened, closed: r.closed, fails: r.fails.length, memory: r.memory, memoryRatio: ratio },
    };
  },
});
