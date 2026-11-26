// file/menu-first-item-chosen: File > #1 (the first item of the File drop-down, menus.tsv: file-1) is chosen by its path; the engine stays idle and the main form stays.
import { defineScenario } from '../../lib/scenario.mjs';
import { MAIN_FORM, menuPath, windowTable } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'file/menu-first-item-chosen',
  covers: ['file/menu-item/file-1'],
  async run({ guest }) {
    const id = await menuPath(guest, ['File', '#1']);
    await sleep(1500);
    const st = await guest.state();
    const main = (await windowTable(guest)).top.some((w) => w.cls === MAIN_FORM);
    return { ok: st.state === 'engine-idle' && main, engineState: st.state, notes: [`chose ${id}; title "${st.title}"`] };
  },
});
