// file/demo-songs-jay-cactus-x-confz-mic-check-ready: the demo song "Jay Cactus x Confz - Mic Check Ready" (Data\Demo projects\Demo songs\Jay Cactus x Confz - Mic Check Ready.flp) is opened by the open box (Ctrl+O): the project loads (its title changes, or the engine logs the file's name), and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: "file/demo-songs-jay-cactus-x-confz-mic-check-ready",
  covers: ["file/demo/demo-songs-jay-cactus-x-confz-mic-check-ready"],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, "Data\\Demo projects\\Demo songs\\Jay Cactus x Confz - Mic Check Ready.flp"));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`DPR 1 only (the file area's demos open at DPR 1: the time of a shard; see displays): Jay Cactus x Confz - Mic Check Ready: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the file's name in the engine log ${r.logHit !== null}`],
      results: { titleChanged: r.titleChanged, logHit: r.logHit !== null, waitedMs: r.waitedMs },
    };
  },
});
