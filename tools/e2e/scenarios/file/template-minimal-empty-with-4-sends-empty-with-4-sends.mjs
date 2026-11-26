// file/template-minimal-empty-with-4-sends-empty-with-4-sends: the template "Empty with 4 sends" (Data\Templates\Minimal\Empty with 4 sends\Empty with 4 sends.flp) is opened by the open box (Ctrl+O): the project loads (its title changes, or the engine logs the file's name), and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: "file/template-minimal-empty-with-4-sends-empty-with-4-sends",
  covers: ["file/template/minimal-empty-with-4-sends-empty-with-4-sends"],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, "Data\\Templates\\Minimal\\Empty with 4 sends\\Empty with 4 sends.flp"));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`Empty with 4 sends: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the file's name in the engine log ${r.logHit !== null}`],
      results: { titleChanged: r.titleChanged, logHit: r.logHit !== null, waitedMs: r.waitedMs },
    };
  },
});
