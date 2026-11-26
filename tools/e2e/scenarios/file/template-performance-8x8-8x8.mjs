// file/template-performance-8x8-8x8: the template "8x8" (Data\Templates\Performance\8x8\8x8.flp) is opened by the open box (Ctrl+O): the project loads (its title changes, or the engine logs the file's name), and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: "file/template-performance-8x8-8x8",
  covers: ["file/template/performance-8x8-8x8"],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, "Data\\Templates\\Performance\\8x8\\8x8.flp"));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`8x8: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the file's name in the engine log ${r.logHit !== null}`],
      results: { titleChanged: r.titleChanged, logHit: r.logHit !== null, waitedMs: r.waitedMs },
    };
  },
});
