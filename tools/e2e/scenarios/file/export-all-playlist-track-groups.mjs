// file/export-all-playlist-track-groups: File > Export > All playlist track groups (the item 10 of the sub-menu) opens its own sub-menu (a drop-down is shown); the engine stays idle.
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, menuPath, newWindows, openLids, POPUP } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'file/export-all-playlist-track-groups',
  covers: ['file/export/all-playlist-track-groups'],
  async run({ guest }) {
    const base = await openLids(guest);
    const id = await menuPath(guest, ['File', '#9', '#10']);
    await sleep(800);
    const pops = (await newWindows(guest, base)).pops.filter((w) => w.cls === POPUP).length;
    const st = await guest.state();
    await closeNew(guest, base);
    return { ok: pops > 0 && st.state === 'engine-idle', engineState: st.state, notes: [`${id}: drop-downs shown ${pops}; state ${st.state}`] };
  },
});
