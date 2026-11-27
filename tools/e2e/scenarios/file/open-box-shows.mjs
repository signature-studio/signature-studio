// file/open-box-shows: Ctrl+O shows the engine's open box (the page's #filedlg-input, or the file manager #fx-input-files). The box is answered with the installer's demo song
// (lib/flow.mjs openProjectFile: the tool takes the browser's chooser, since a headless browser cancels a chooser at once and Escape does not close the box); the box is gone after the answer and the song opens.
import path from 'node:path';
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

const BOX = "document.getElementById('fx-input-files') ? 'explorer' : null";
const DEMO = path.join('Data', 'Demo projects', 'Demo songs', 'DEDZII - NIGHT TERROR.flp');

export default defineScenario({
  id: 'file/open-box-shows',
  covers: ['file/open/ctrl-o'],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, DEMO));
    const gone = (await ctx.guest.ev(BOX)) === null;
    return {
      ok: r.box !== null && gone === true && r.opened === true,
      engineState: r.state,
      notes: [`box ${r.box} shown by Ctrl+O; closed after the answer ${gone}; song open ${r.opened} (title "${r.title}", before "${r.titleBefore}")`],
      results: { titleChanged: r.titleChanged, waitedMs: r.waitedMs },
    };
  },
});
