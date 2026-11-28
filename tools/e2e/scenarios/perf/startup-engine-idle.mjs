// perf/startup-engine-idle: the start-up (navigation to engine-idle, and to the engine's first idle mark) in new pages: the median of 3 starts is judged against the same run's
// reference (the median of 3 starts before them; not golden/perf/baseline.tsv, Issue #1509): the ratio at most 1.5.
import { defineScenario } from '../../lib/scenario.mjs';
import { measureStartup } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/startup-engine-idle',
  covers: ['perf/startup/engine-idle', 'perf/startup/first-idle'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const r = await measureStartup(ctx);
    return { ok: r.ok, engineState: r.state, notes: r.notes, results: r.results };
  },
});
