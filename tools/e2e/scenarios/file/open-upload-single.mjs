// file/open-upload-single: one demo song uploaded into the open box (Ctrl+O): the project opens with its own title; the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/open-upload-single',
  covers: ['file/open/upload-single'],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, 'Data\\Demo projects\\Demo songs\\DEDZII - NIGHT TERROR.flp'));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`box ${r.box}: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the name in the log ${r.logHit !== null}`],
      results: { titleChanged: r.titleChanged, waitedMs: r.waitedMs },
    };
  },
});
