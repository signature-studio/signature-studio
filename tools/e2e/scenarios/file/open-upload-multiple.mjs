// file/open-upload-multiple: two projects given to the open box at once (Ctrl+O; the box takes several files): the box is answered, a project opens, and the engine stays idle.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openFilesAtOnce } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/open-upload-multiple',
  covers: ['file/open/upload-multiple'],
  async run(ctx) {
    const files = [dataPath(ctx.repo, 'Data\\Demo projects\\NewStuff.flp'), dataPath(ctx.repo, 'Data\\Demo projects\\Performance mode\\Drumloop Masher.flp')];
    const r = await openFilesAtOnce(ctx, files);
    return {
      ok: r.given === true && r.opened === true,
      engineState: r.state,
      notes: [`${r.files} files given (box ${r.box}, given ${r.given}${r.error ? `, error ${r.error}` : ''}): title "${r.title}" (before "${r.titleBefore}"), state ${r.state}`],
    };
  },
});
