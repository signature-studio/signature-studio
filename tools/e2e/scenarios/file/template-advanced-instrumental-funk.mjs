// file/template-advanced-instrumental-funk: the template "Funk" (Data\Templates\Advanced\Instrumental\Funk.flp) is opened by the open box (Ctrl+O): the project loads (its title changes, or the engine logs the file's name), and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: "file/template-advanced-instrumental-funk",
  covers: ["file/template/advanced-instrumental-funk"],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, "Data\\Templates\\Advanced\\Instrumental\\Funk.flp"));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`Funk: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the file's name in the engine log ${r.logHit !== null}`],
      results: { titleChanged: r.titleChanged, logHit: r.logHit !== null, waitedMs: r.waitedMs },
    };
  },
});
