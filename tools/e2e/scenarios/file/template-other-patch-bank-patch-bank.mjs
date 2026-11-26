// file/template-other-patch-bank-patch-bank: the template "Patch bank" (Data\Templates\Other\Patch bank\Patch bank.flp) is opened by the open box (Ctrl+O): the project loads (its title changes, or the engine logs the file's name), and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: "file/template-other-patch-bank-patch-bank",
  covers: ["file/template/other-patch-bank-patch-bank"],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, "Data\\Templates\\Other\\Patch bank\\Patch bank.flp"));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`Patch bank: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the file's name in the engine log ${r.logHit !== null}`],
      results: { titleChanged: r.titleChanged, logHit: r.logHit !== null, waitedMs: r.waitedMs },
    };
  },
});
