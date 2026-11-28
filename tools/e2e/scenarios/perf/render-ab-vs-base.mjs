// perf/render-ab-vs-base: the A/B of the rendering: the least one-second window of the rAF counter with no input, judged against the same run's quiet reference (the A/B of two builds cannot run in one job; Issue #1509).
import { defineScenario } from '../../lib/scenario.mjs';
import { measureFps } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/render-ab-vs-base',
  covers: ['perf/ab/render'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const r = await measureFps(ctx, 'render.minFps.idle', 8000, null);
    return { ok: r.ok, engineState: r.state, notes: ['A/B: the reference is the same run (no golden table; Issue #1509)'].concat(r.notes), results: r.results };
  },
});
