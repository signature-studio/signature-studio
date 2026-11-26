// file/save-as-dialog: a demo song is open; File > Save as (the item 5 of the File drop-down, menus.tsv file-5): the engine's Save dialog shows (the file manager in selection mode);
// its Cancel closes it, and the engine stays idle.
import { defineScenario } from '../../lib/scenario.mjs';
import { menuPath, openLids, closeNew } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';
import { clickElement, dataPath, dialogState, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/save-as-dialog',
  covers: ['file/menu-item/file-5', 'file/save/save-as'],
  async run(ctx) {
    const { guest, repo } = ctx;
    const op = await openDataFile(ctx, dataPath(repo, 'Data\\Demo projects\\NewStuff.flp'));
    if (!op.opened) return { ok: false, engineState: op.state, notes: [`the demo did not open (state ${op.state})`] };
    const base = await openLids(guest);
    const id = await menuPath(guest, ['File', '#5']);
    let shown = false;
    const t0 = Date.now();
    while (Date.now() - t0 < 15000) {
      if ((await dialogState(guest)).open) { shown = true; break; }
      await sleep(300);
    }
    const cancelled = shown ? await clickElement(guest, '#fx-close') : false;
    let closed = false;
    if (cancelled) {
      const t1 = Date.now();
      while (Date.now() - t1 < 10000) {
        if (!(await dialogState(guest)).open) { closed = true; break; }
        await sleep(300);
      }
    }
    await closeNew(guest, base);
    const st = await guest.state();
    return {
      ok: shown && closed && st.state === 'engine-idle',
      engineState: st.state,
      notes: [`${id}: the Save dialog shown ${shown}; Cancel pressed ${cancelled}; closed ${closed}; state ${st.state}`],
    };
  },
});
