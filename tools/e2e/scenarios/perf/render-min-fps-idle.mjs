// perf/render-min-fps-idle: with no input for 8 s, the least one-second window of the page's rAF counter, judged against the baseline (the fall at most 1.5 times).
import { defineScenario } from '../../lib/scenario.mjs';
import { measureFps } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/render-min-fps-idle',
  covers: ['perf/render/min-fps-idle'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const r = await measureFps(ctx, 'render.minFps.idle', 8000, null);
    return { ok: r.ok, engineState: r.state, notes: r.notes, results: r.results };
  },
});
