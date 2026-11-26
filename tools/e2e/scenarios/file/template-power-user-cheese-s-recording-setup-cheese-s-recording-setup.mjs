// file/template-power-user-cheese-s-recording-setup-cheese-s-recording-setup: the template "Cheese's recording setup" (Data\Templates\Power User\Cheese's recording setup\Cheese's recording setup.flp) is opened by the open box (Ctrl+O): the project loads (its title changes, or the engine logs the file's name), and the engine stays idle with no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: "file/template-power-user-cheese-s-recording-setup-cheese-s-recording-setup",
  covers: ["file/template/power-user-cheese-s-recording-setup-cheese-s-recording-setup"],
  async run(ctx) {
    const r = await openDataFile(ctx, dataPath(ctx.repo, "Data\\Templates\\Power User\\Cheese's recording setup\\Cheese's recording setup.flp"));
    return {
      ok: r.opened === true,
      engineState: r.state,
      notes: [`Cheese's recording setup: title "${r.title}" (before "${r.titleBefore}"), state ${r.state}, the file's name in the engine log ${r.logHit !== null}`],
      results: { titleChanged: r.titleChanged, logHit: r.logHit !== null, waitedMs: r.waitedMs },
    };
  },
});
