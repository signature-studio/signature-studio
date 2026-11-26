// file/template-other-blocks-blocks: the template "Blocks" (Data\Templates\Other\Blocks\Blocks.flp) is opened by the open box (Ctrl+O): the project loads (its title changes, or the engine logs the file's name), and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: "file/template-other-blocks-blocks",
  covers: ["file/template/other-blocks-blocks"],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, "Data\\Templates\\Other\\Blocks\\Blocks.flp"));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`Blocks: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the file's name in the engine log ${r.logHit !== null}`],
      results: { titleChanged: r.titleChanged, logHit: r.logHit !== null, waitedMs: r.waitedMs },
    };
  },
});
