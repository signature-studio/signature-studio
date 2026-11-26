// file/new-ctrl-n-twice: New (Ctrl+N) twice in a row: the engine stays idle, the main form stays, and no window is left open after either New.
import { defineScenario } from '../../lib/scenario.mjs';
import { MAIN_FORM, resetProject, windowTable } from '../../lib/common.mjs';

export default defineScenario({
  id: 'file/new-ctrl-n-twice',
  covers: ['file/new/ctrl-n'],
  async run({ guest }) {
    const first = await resetProject(guest);
    const second = await resetProject(guest);
    const st = await guest.state();
    const main = (await windowTable(guest)).top.some((w) => w.cls === MAIN_FORM);
    return {
      ok: st.state === 'engine-idle' && main && first.left.length === 0 && second.left.length === 0,
      engineState: st.state,
      notes: [`title "${st.title}", main form ${main}, windows left after New ${first.left.length} and ${second.left.length}`],
    };
  },
});
