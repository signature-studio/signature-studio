// file/demo-songs-purps-on-the-beat-feat-luclover-stroll-up: the demo song "Purps On The Beat Feat Luclover - Stroll Up" (Data\Demo projects\Demo songs\Purps On The Beat Feat Luclover - Stroll Up.flp) is opened by the open box (Ctrl+O): the project loads (its title changes, or the engine logs the file's name), and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: "file/demo-songs-purps-on-the-beat-feat-luclover-stroll-up",
  covers: ["file/demo/demo-songs-purps-on-the-beat-feat-luclover-stroll-up"],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, "Data\\Demo projects\\Demo songs\\Purps On The Beat Feat Luclover - Stroll Up.flp"));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`DPR 1 only (the file area's demos open at DPR 1: the time of a shard; see displays): Purps On The Beat Feat Luclover - Stroll Up: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the file's name in the engine log ${r.logHit !== null}`],
      results: { titleChanged: r.titleChanged, logHit: r.logHit !== null, waitedMs: r.waitedMs },
    };
  },
});
