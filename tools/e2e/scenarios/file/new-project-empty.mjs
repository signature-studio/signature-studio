// file/new-project-empty: after File > New (the runner's reset: Ctrl+N), the engine is idle, the main form is in the window table and no drop-down is left open.
import { defineScenario } from '../../lib/scenario.mjs';
import { MAIN_FORM, newWindows, openLids, windowTable } from '../../lib/common.mjs';

export default defineScenario({
  id: 'file/new-project-empty',
  covers: ['file/template/minimal-empty-empty'],
  async run({ guest }) {
    const t = await windowTable(guest);
    const main = t.top.some((w) => w.cls === MAIN_FORM);
    const st = await guest.state();
    const base = await openLids(guest);
    const pops = (await newWindows(guest, base)).pops.length;
    return { ok: st.state === 'engine-idle' && main && pops === 0, engineState: st.state, notes: [`title "${st.title}", main form ${main}, drop-downs ${pops}`] };
  },
});
