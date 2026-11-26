// file/open-folder: a folder given to the open box (Ctrl+O). A folder is not a project: the engine stays idle and does not stop; what the box did is in the notes.
// When the box is still open after the attempt (the folder was refused), it is answered with a demo song, so that the next scenario starts clean.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, giveFiles, openFilesAtOnce, waitBox } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/open-folder',
  covers: ['file/open/folder'],
  async run(ctx) {
    const { guest, cdp, repo } = ctx;
    const r = await openFilesAtOnce(ctx, [dataPath(repo, 'Data\\Demo projects\\Demo songs')]);
    let answered = false;
    if ((await waitBox(guest, 1500)) === 'filedlg') {
      answered = (await giveFiles(guest, cdp, [dataPath(repo, 'Data\\Demo projects\\NewStuff.flp')])).given === true;
    }
    const st = await guest.state();
    return {
      ok: st.state === 'engine-idle',
      engineState: st.state,
      notes: [`folder given: box ${r.box}, given ${r.given}${r.error ? `, refused: ${r.error.slice(0, 160)}` : ''}; the box answered after it ${answered}; title "${st.title}", state ${st.state}`],
    };
  },
});
