// file/open-folder: a folder given to the engine's Open window (File > Open; Issue #1519: the window takes a folder into its list, as an upload). A folder is not a project: the window
// refuses it (it says Choose a file and stays), the scenario closes it with its Cancel, and the engine stays idle and does not stop; what the window did is in the notes.
import { defineScenario } from '../../lib/scenario.mjs';
import { clickElement, dataPath, openFilesAtOnce } from '../../lib/file.mjs';
import { sleep, WINDOW } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'file/open-folder',
  covers: ['file/open/folder'],
  async run(ctx) {
    const { guest, repo } = ctx;
    const folder = dataPath(repo, 'Data\\Demo projects\\Demo songs');
    const r = await openFilesAtOnce(ctx, [folder]);
    let closed = false;
    if (r.refused) {
      await clickElement(guest, '#fx-close');
      const t0 = Date.now();
      while (Date.now() - t0 < 10000) {
        if ((await guest.ev(WINDOW)) === null) { closed = true; break; }
        await sleep(300);
      }
    }
    const st = await guest.state();
    return {
      ok: r.given === true && r.refused === true && closed && st.state === 'engine-idle',
      engineState: st.state,
      notes: [`folder given ${r.given}${r.error ? `, error ${r.error.slice(0, 160)}` : ''}; the window refused it ${r.refused === true}; closed with Cancel ${closed}; title "${st.title}", state ${st.state}`],
    };
  },
});
