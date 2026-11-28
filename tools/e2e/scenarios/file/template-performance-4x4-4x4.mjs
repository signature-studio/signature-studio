// file/template-performance-4x4-4x4: the template "4x4" (Data\Templates\Performance\4x4\4x4.flp) is opened by the open box (Ctrl+O): the project loads (its title changes, or the engine logs the file's name), and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile, openFacts } from '../../lib/file.mjs';

export default defineScenario({
  id: "file/template-performance-4x4-4x4",
  covers: ["file/template/performance-4x4-4x4"],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, "Data\\Templates\\Performance\\4x4\\4x4.flp"));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`4x4: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the file's name in the engine log ${r.logHit !== null}; ${openFacts(r)}`],
      results: { titleChanged: r.titleChanged, logHit: r.logHit !== null, boxLeft: r.boxLeft, mainForm: r.mainForm, problemsAnswers: r.problemsAnswers, waitedMs: r.waitedMs },
    };
  },
});
