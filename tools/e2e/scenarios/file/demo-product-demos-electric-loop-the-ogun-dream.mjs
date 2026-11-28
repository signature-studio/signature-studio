// file/demo-product-demos-electric-loop-the-ogun-dream: the demo song "Electric Loop - The Ogun Dream" (Data\Demo projects\Product demos\Electric Loop - The Ogun Dream.flp) is opened by the open box (Ctrl+O): the project loads (its title changes, or the engine logs the file's name), and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: "file/demo-product-demos-electric-loop-the-ogun-dream",
  covers: ["file/demo/product-demos-electric-loop-the-ogun-dream"],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, "Data\\Demo projects\\Product demos\\Electric Loop - The Ogun Dream.flp"));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`DPR 1 only (the file area's demos open at DPR 1: the time of a shard; see displays): Electric Loop - The Ogun Dream: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the file's name in the engine log ${r.logHit !== null}`],
      results: { titleChanged: r.titleChanged, logHit: r.logHit !== null, waitedMs: r.waitedMs },
    };
  },
});
