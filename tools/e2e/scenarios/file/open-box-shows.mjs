// file/open-box-shows: Ctrl+O shows the engine's open box (the page's #filedlg-input, or the file manager #fx-input-files). The box is answered with the installer's demo song
// (lib/flow.mjs openProjectFile: the tool takes the browser's chooser, since a headless browser cancels a chooser at once and Escape does not close the box); the box is gone after the answer and the song opens.
import path from 'node:path';
import { defineScenario } from '../../lib/scenario.mjs';
import { dismissProblems, openProjectFile, waitProjectOpen } from '../../lib/flow.mjs';

const BOX = "document.getElementById('filedlg-input') ? 'filedlg' : (document.getElementById('fx-input-files') ? 'explorer' : null)";
const DEMO = path.join('Data', 'Demo projects', 'Demo songs', 'DEDZII - NIGHT TERROR.flp');

export default defineScenario({
  id: 'file/open-box-shows',
  covers: ['file/menu-item/file-2'],
  async run({ guest, cdp, repo }) {
    await dismissProblems(guest);
    const before = await guest.state();
    let box = null;
    try {
      box = await openProjectFile(guest, cdp, path.join(repo, 'lift', 'out-app', 'host', 'resources', DEMO), () => {});
    } catch (e) {
      return { ok: false, engineState: (await guest.state())?.state ?? null, notes: [`no open box: ${String(e?.message ?? e).slice(0, 200)}`] };
    }
    const gone = await guest.waitFor(async () => (await guest.ev(BOX)) === null, 20000, 'the open box to close');
    const end = await waitProjectOpen(guest, before?.title ?? null, 120000);
    return {
      ok: box !== null && gone === true && end.opened === true,
      engineState: end.state ?? null,
      notes: [`box ${box} shown by Ctrl+O; closed after the answer ${gone === true}; song open ${end.opened} (title "${end.title}")`],
    };
  },
});
