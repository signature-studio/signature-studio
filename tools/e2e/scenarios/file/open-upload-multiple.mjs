// file/open-upload-multiple: two projects are uploaded into the engine's Open window at once (File > Open / Ctrl+O; Issue #1519). The window lists both; FL's Open allows one
// file (the request has no FOS_ALLOWMULTISELECT), so the first is chosen and Open opens it. The project opens, and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openFilesAtOnce } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/open-upload-multiple',
  covers: ['file/open/upload-multiple'],
  async run(ctx) {
    // the chosen one is the Drumloop (not the song that an earlier scenario of the page may have open: the title would not change)
    const files = [dataPath(ctx.repo, 'Data\\Demo projects\\Performance mode\\Drumloop Masher.flp'), dataPath(ctx.repo, 'Data\\Demo projects\\NewStuff.flp')];
    const r = await openFilesAtOnce(ctx, files, { choose: 1 });
    return {
      ok: r.given === true && r.opened === true,
      engineState: r.state,
      notes: [`${r.files} files uploaded into the window, ${r.chosen} chosen (box ${r.box}, given ${r.given}${r.error ? `, error ${r.error}` : ''}): title "${r.title}" (before "${r.titleBefore}"), state ${r.state}`],
    };
  },
});
