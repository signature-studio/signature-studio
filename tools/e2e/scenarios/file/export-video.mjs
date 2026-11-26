// file/export-video: File > Export > Video file (the item 11 of the sub-menu): the engine's video dialog or a window opens; the scenario cancels it; the engine stays idle.
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, menuPath, openLids } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';
import { clickElement, dialogState, freshTitles } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/export-video',
  covers: ['file/export/video'],
  async run({ guest }) {
    const base = await openLids(guest);
    const id = await menuPath(guest, ['File', '#9', '#11']);
    let seen = { windows: [], dialog: false };
    const t0 = Date.now();
    while (Date.now() - t0 < 15000) {
      seen = { windows: await freshTitles(guest, base), dialog: (await dialogState(guest)).open };
      if (seen.windows.length > 0 || seen.dialog) break;
      await sleep(300);
    }
    if (seen.dialog) await clickElement(guest, '#fx-close');
    await sleep(500);
    await closeNew(guest, base);
    const st = await guest.state();
    return {
      ok: (seen.windows.length > 0 || seen.dialog) && st.state === 'engine-idle',
      engineState: st.state,
      notes: [`${id}: new windows ${JSON.stringify(seen.windows).slice(0, 200)}; dialog ${seen.dialog}; state ${st.state}`],
    };
  },
});
