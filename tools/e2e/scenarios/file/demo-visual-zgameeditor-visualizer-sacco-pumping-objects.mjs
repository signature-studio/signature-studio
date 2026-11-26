// file/demo-visual-zgameeditor-visualizer-sacco-pumping-objects: the demo song "Sacco - Pumping objects" (Data\Demo projects\Visual\ZGameEditor Visualizer\Sacco - Pumping objects.flp) is opened by the open box (Ctrl+O): the project loads (its title changes, or the engine logs the file's name), and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: "file/demo-visual-zgameeditor-visualizer-sacco-pumping-objects",
  covers: ["file/demo/visual-zgameeditor-visualizer-sacco-pumping-objects"],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, "Data\\Demo projects\\Visual\\ZGameEditor Visualizer\\Sacco - Pumping objects.flp"));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`DPR 1 only (the file area's demos open at DPR 1: the time of a shard; see displays): Sacco - Pumping objects: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the file's name in the engine log ${r.logHit !== null}`],
      results: { titleChanged: r.titleChanged, logHit: r.logHit !== null, waitedMs: r.waitedMs },
    };
  },
});
