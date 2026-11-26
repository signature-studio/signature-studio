// perf/startup-engine-idle: the start-up (navigation to engine-idle, and to the engine's first idle mark) in 3 new pages: the median is judged against the baseline (the ratio at most 1.5).
import { defineScenario } from '../../lib/scenario.mjs';
import { measureStartup } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/startup-engine-idle',
  covers: ['perf/startup/engine-idle', 'perf/startup/first-idle'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const r = await measureStartup(ctx, 'base');
    return { ok: r.ok, engineState: r.state, notes: r.notes, results: r.results };
  },
});
