// file/save-as-template: a demo song is open; File > Save as template (the item 7 of the File drop-down, menus.tsv file-7): the engine asks for the template (a window, the dialog,
// or a file listed); the engine stays idle, and the scenario closes what it left.
import { defineScenario } from '../../lib/scenario.mjs';
import { menuPath, openLids, closeNew } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';
import { clickElement, dataPath, dialogState, freshTitles, openDataFile, savedFiles } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/save-as-template',
  covers: ['file/menu-item/file-7', 'file/save/as-template'],
  async run(ctx) {
    const { guest, repo } = ctx;
    const op = await openDataFile(ctx, dataPath(repo, 'Data\\Demo projects\\NewStuff.flp'));
    if (!op.opened) return { ok: false, engineState: op.state, notes: [`the demo did not open (state ${op.state})`] };
    const base = await openLids(guest);
    const listed = (await savedFiles(guest)).length;
    const id = await menuPath(guest, ['File', '#7']);
    let seen = { windows: [], dialog: false, listed: 0 };
    const t0 = Date.now();
    while (Date.now() - t0 < 15000) {
      seen = { windows: await freshTitles(guest, base), dialog: (await dialogState(guest)).open, listed: (await savedFiles(guest)).length - listed };
      if (seen.windows.length > 0 || seen.dialog || seen.listed > 0) break;
      await sleep(300);
    }
    if (seen.dialog) await clickElement(guest, '#fx-close');
    await sleep(500);
    await closeNew(guest, base);
    const st = await guest.state();
    return {
      ok: (seen.windows.length > 0 || seen.dialog || seen.listed > 0) && st.state === 'engine-idle',
      engineState: st.state,
      notes: [`${id}: windows ${JSON.stringify(seen.windows).slice(0, 200)}; dialog ${seen.dialog}; files listed ${seen.listed}; state ${st.state}`],
    };
  },
});
