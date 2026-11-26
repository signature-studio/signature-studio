// file/export-master: File > Export > Master (the item 7 of the sub-menu): the mastering window opens (its title is in the window table); the engine stays idle.
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, menuPath, openLids } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';
import { freshTitles } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/export-master',
  covers: ['file/export/master'],
  async run({ guest }) {
    const base = await openLids(guest);
    const id = await menuPath(guest, ['File', '#9', '#7']);
    await sleep(3000);
    const fresh = await freshTitles(guest, base);
    const st = await guest.state();
    await closeNew(guest, base);
    return { ok: fresh.length > 0 && st.state === 'engine-idle', engineState: st.state, notes: [`${id}: new windows ${JSON.stringify(fresh)}; state ${st.state}`] };
  },
});
