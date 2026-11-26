// file/export-project-data-files: File > Export > Project data files (the item 12 of the sub-menu): the project's data files are shown in the file manager (its list has rows),
// or a window of the data files is shown; the scenario closes what it opened; the engine stays idle.
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, menuPath, openLids } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';
import { clickElement, dialogState, freshTitles } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/export-project-data-files',
  covers: ['file/export/project-data-files'],
  async run({ guest }) {
    const base = await openLids(guest);
    const id = await menuPath(guest, ['File', '#9', '#12']);
    let seen = { windows: [], dialog: false, rows: 0 };
    const t0 = Date.now();
    while (Date.now() - t0 < 15000) {
      const dlg = await dialogState(guest);
      seen = { windows: await freshTitles(guest, base), dialog: dlg.open, rows: dlg.open ? await guest.ev("document.querySelectorAll('#fx-list [data-name]').length") : 0 };
      if (seen.windows.length > 0 || seen.dialog) break;
      await sleep(300);
    }
    await sleep(800);
    if (seen.dialog) {
      seen.rows = await guest.ev("document.querySelectorAll('#fx-list [data-name]').length");
      await clickElement(guest, '#fx-close');
    }
    await sleep(500);
    await closeNew(guest, base);
    const st = await guest.state();
    return {
      ok: (seen.windows.length > 0 || (seen.dialog && seen.rows > 0)) && st.state === 'engine-idle',
      engineState: st.state,
      notes: [`${id}: new windows ${JSON.stringify(seen.windows).slice(0, 200)}; file manager shown ${seen.dialog} with ${seen.rows} rows; state ${st.state}`],
      results: { rows: seen.rows },
    };
  },
});
