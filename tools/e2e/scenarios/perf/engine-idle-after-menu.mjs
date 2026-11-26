// perf/engine-idle-after-menu: after a drop-down is opened and closed, the engine is idle again (not running) within 5 s.
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, openLids, openMenu } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'perf/engine-idle-after-menu',
  covers: [],
  async run({ guest }) {
    const base = await openLids(guest);
    await openMenu(guest, 'File');
    await closeNew(guest, base);
    let st = null;
    for (let i = 0; i < 17; i++) {
      st = await guest.state();
      if (st.state === 'engine-idle') break;
      await sleep(300);
    }
    return { ok: st.state === 'engine-idle', engineState: st.state, notes: [`state ${st.state} after the close`] };
  },
});
