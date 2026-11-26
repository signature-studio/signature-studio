// file/new-after-demo: a demo song is open, then New (Ctrl+N): the title changes to the new project's, the engine stays idle, and no window is left open.
import { defineScenario } from '../../lib/scenario.mjs';
import { resetProject } from '../../lib/common.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/new-after-demo',
  covers: ['file/new/ctrl-n'],
  async run(ctx) {
    const op = await openDataFile(ctx, dataPath(ctx.repo, 'Data\\Demo projects\\NewStuff.flp'));
    const r = await resetProject(ctx.guest);
    const st = await ctx.guest.state();
    return {
      ok: op.opened === true && st.state === 'engine-idle' && st.title !== op.title && r.left.length === 0,
      engineState: st.state,
      notes: [`demo title "${op.title}", after New title "${st.title}", windows left ${r.left.length}; error ${op.error ?? "none"}; windows ${JSON.stringify((await ctx.guest.windows()).map((w) => `${w.cls}|${w.title}`))}`],
    };
  },
});
