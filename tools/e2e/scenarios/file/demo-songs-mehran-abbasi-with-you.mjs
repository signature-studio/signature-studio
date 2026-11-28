// file/demo-songs-mehran-abbasi-with-you: the demo song "Mehran Abbasi - With You" (Data\Demo projects\Demo songs\Mehran Abbasi - With You.flp) is opened by the open box (Ctrl+O): the project loads (its title changes, or the engine logs the file's name), and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: "file/demo-songs-mehran-abbasi-with-you",
  covers: ["file/demo/demo-songs-mehran-abbasi-with-you"],
  displays: ['4k-dpr1'],
  // found by the run of #1488: the engine stops (exit 134, SIGABRT) when the demo is opened
  expectFail: 'new',
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, "Data\\Demo projects\\Demo songs\\Mehran Abbasi - With You.flp"));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`DPR 1 only (the file area's demos open at DPR 1: the time of a shard; see displays): Mehran Abbasi - With You: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the file's name in the engine log ${r.logHit !== null}`],
      results: { titleChanged: r.titleChanged, logHit: r.logHit !== null, waitedMs: r.waitedMs },
    };
  },
});
