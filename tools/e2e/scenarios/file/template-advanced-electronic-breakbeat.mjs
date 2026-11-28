// file/template-advanced-electronic-breakbeat: the template "Breakbeat" (Data\Templates\Advanced\Electronic\Breakbeat.flp) is opened by the open box (Ctrl+O): the project loads (its title changes, or the engine logs the file's name), and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile, openFacts } from '../../lib/file.mjs';

export default defineScenario({
  id: "file/template-advanced-electronic-breakbeat",
  covers: ["file/template/advanced-electronic-breakbeat"],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, "Data\\Templates\\Advanced\\Electronic\\Breakbeat.flp"));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`Breakbeat: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the file's name in the engine log ${r.logHit !== null}; ${openFacts(r)}`],
      results: { titleChanged: r.titleChanged, logHit: r.logHit !== null, boxLeft: r.boxLeft, mainForm: r.mainForm, problemsAnswers: r.problemsAnswers, waitedMs: r.waitedMs },
    };
  },
});
