// perf/render-min-fps-new-project: File > New (Ctrl+N) three times; the least one-second window of the rAF counter over the time, judged against the baseline.
import { defineScenario } from '../../lib/scenario.mjs';
import { measureFps } from '../../lib/perf.mjs';
import { resetProject } from '../../lib/common.mjs';

export default defineScenario({
  id: 'perf/render-min-fps-new-project',
  covers: ['perf/render/min-fps-new-project'],
  displays: ['4k-dpr1'],
  // Issue #1497 run 2 (this runner): 1.946 rAF/s against the baseline's 5.707 (ratio 0.341, below 1/1.5). The least 1 s window of the resets is the unstable part (see the report)
  expectFail: 'new',
  async run(ctx) {
    const resets = async () => { for (let i = 0; i < 3; i++) await resetProject(ctx.guest); };
    const r = await measureFps(ctx, 'render.minFps.newProject', 1000, resets);
    return { ok: r.ok, engineState: r.state, notes: ['three File > New (Ctrl+N); the window is the resets'].concat(r.notes), results: r.results };
  },
});
