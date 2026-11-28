// perf/startup-ab-vs-base: the A/B of the start-up: 3 starts, judged against the same run's reference (3 starts before them), not golden/perf/ab-base.tsv (Issue #1509).
import { defineScenario } from '../../lib/scenario.mjs';
import { measureStartup } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/startup-ab-vs-base',
  covers: ['perf/ab/startup'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const r = await measureStartup(ctx);
    return { ok: r.ok, engineState: r.state, notes: ['A/B: the base is the first 3 starts of this run (two sets on this machine; the ab-base table is not read)'].concat(r.notes), results: r.results };
  },
});
