// file/template-utility-surround-mix-surround-mix: the template "Surround mix" (Data\Templates\Utility\Surround mix\Surround mix.flp) is opened by the open box (Ctrl+O): the project loads (its title changes, or the engine logs the file's name), and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: "file/template-utility-surround-mix-surround-mix",
  covers: ["file/template/utility-surround-mix-surround-mix"],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, "Data\\Templates\\Utility\\Surround mix\\Surround mix.flp"));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`Surround mix: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the file's name in the engine log ${r.logHit !== null}`],
      results: { titleChanged: r.titleChanged, logHit: r.logHit !== null, waitedMs: r.waitedMs },
    };
  },
});
