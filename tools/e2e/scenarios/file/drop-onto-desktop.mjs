// file/drop-onto-desktop: a demo file is dragged onto the desktop (CDP drag with the file's path). The file manager window is the drop target in the page (explorer.ts);
// the desktop itself does not open a dropped file: the engine stays idle and the title does not change.
import { defineScenario } from '../../lib/scenario.mjs';
import { sleep } from '../../lib/flow.mjs';
import { dataPath, dropFile } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/drop-onto-desktop',
  covers: ['file/drop/desktop'],
  async run({ guest, repo }) {
    const before = await guest.state();
    await guest.focusDesktop();
    await dropFile(guest, dataPath(repo, 'Data\\Demo projects\\NewStuff.flp'), 1000, 800);
    await sleep(6000);
    const st = await guest.state();
    return {
      ok: st.state === 'engine-idle' && st.title === before.title,
      engineState: st.state,
      notes: [`dropped on the desktop: title "${st.title}" (before "${before.title}"), state ${st.state}`],
    };
  },
});
