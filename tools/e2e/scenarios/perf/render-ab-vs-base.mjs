// perf/render-ab-vs-base: the A/B of the rendering: the least one-second window of the rAF counter with no input, judged against the base of the change (golden/perf/ab-base.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { measureFps } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/render-ab-vs-base',
  covers: ['perf/ab/render'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const r = await measureFps(ctx, 'render.minFps.idle', 8000, null, 'ab');
    return { ok: r.ok, engineState: r.state, notes: ['A/B: the base is golden/perf/ab-base.tsv'].concat(r.notes), results: r.results };
  },
});
