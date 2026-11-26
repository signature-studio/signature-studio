// file/exit-not-chosen: File > Exit (the 10th item of the File drop-down, menus.tsv file-10) is reached by Down presses and is not chosen: the drop-down stays open
// with the engine idle; Escape closes it. (Choosing Exit ends the engine, so the scenario does not choose it.)
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, newWindows, openLids, openMenu, POPUP } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'file/exit-not-chosen',
  covers: ['file/menu-item/file-10'],
  async run({ guest }) {
    const base = await openLids(guest);
    await openMenu(guest, 'File');
    for (let k = 0; k < 10; k++) {
      await guest.key('ArrowDown', 'ArrowDown', 40);
      await sleep(120);
    }
    await sleep(500);
    const pops = (await newWindows(guest, base)).pops.filter((w) => w.cls === POPUP).length;
    const st = await guest.state();
    await closeNew(guest, base);
    return { ok: pops > 0 && st.state === 'engine-idle', engineState: st.state, notes: [`Down x10 from File: drop-downs shown ${pops}; state ${st.state}`] };
  },
});
