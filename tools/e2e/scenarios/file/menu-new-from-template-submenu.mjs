// file/menu-new-from-template-submenu: File > New from template (the item 2 of the File drop-down, menus.tsv file-2) opens its sub-menu of the template categories
// (a drop-down is shown); the engine stays idle. The templates themselves are the template scenarios (file/template-*), which open the installer's files.
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, menuPath, newWindows, openLids, POPUP } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'file/menu-new-from-template-submenu',
  covers: ['file/menu-item/file-2'],
  async run({ guest }) {
    const base = await openLids(guest);
    const id = await menuPath(guest, ['File', '#2']);
    await sleep(1000);
    const pops = (await newWindows(guest, base)).pops.filter((w) => w.cls === POPUP).length;
    const st = await guest.state();
    await closeNew(guest, base);
    return { ok: pops > 0 && st.state === 'engine-idle', engineState: st.state, notes: [`${id}: drop-downs shown ${pops} (the template categories); state ${st.state}`] };
  },
});
