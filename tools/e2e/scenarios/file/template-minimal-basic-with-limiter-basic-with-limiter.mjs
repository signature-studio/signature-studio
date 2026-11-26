// file/template-minimal-basic-with-limiter-basic-with-limiter: the template "Basic with limiter" (Data\Templates\Minimal\Basic with limiter\Basic with limiter.flp) is opened by the open box (Ctrl+O): the project loads (its title changes, or the engine logs the file's name), and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: "file/template-minimal-basic-with-limiter-basic-with-limiter",
  covers: ["file/template/minimal-basic-with-limiter-basic-with-limiter"],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, "Data\\Templates\\Minimal\\Basic with limiter\\Basic with limiter.flp"));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`Basic with limiter: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the file's name in the engine log ${r.logHit !== null}`],
      results: { titleChanged: r.titleChanged, logHit: r.logHit !== null, waitedMs: r.waitedMs },
    };
  },
});
