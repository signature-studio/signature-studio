// file/new-after-demo: a demo song is open, then New (Ctrl+N): the title changes to the new project's, the engine stays idle, and no window is left open.
// expectFail new: Ctrl+N shows the trial window (TInAppShopForm) and, once it is closed, the title stays the demo's (it did not change in 8 s). After a template or the
// default project the same Ctrl+N makes the new project (Empty_2, Project_2), so the defect is the demo's New, not the box.
import { defineScenario } from '../../lib/scenario.mjs';
import { resetProject } from '../../lib/common.mjs';
import { dataPath, openDataFile, openFacts } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/new-after-demo',
  covers: ['file/new/ctrl-n'],
  expectFail: 'new',
  async run(ctx) {
    const op = await openDataFile(ctx, dataPath(ctx.repo, 'Data\\Demo projects\\NewStuff.flp'));
    const r = await resetProject(ctx.guest);
    const st = await ctx.guest.state();
    return {
      ok: op.opened === true && st.state === 'engine-idle' && st.title !== op.title && r.left.length === 0,
      engineState: st.state,
      notes: [`demo opened ${op.opened} (title "${op.title}", before "${op.titleBefore}"); ${openFacts(op)}; New form ${r.form}, after New title "${st.title}", windows left ${r.left.length}; error ${op.error ?? "none"}; windows ${JSON.stringify((await ctx.guest.windows()).map((w) => `${w.cls}|${w.title}`))}`],
    };
  },
});
