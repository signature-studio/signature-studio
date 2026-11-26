// perf/startup-ab-vs-base: the A/B of the start-up: the same 3 starts, judged against the base of the change (golden/perf/ab-base.tsv), not the baseline.
import { defineScenario } from '../../lib/scenario.mjs';
import { measureStartup } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/startup-ab-vs-base',
  covers: ['perf/ab/startup'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const r = await measureStartup(ctx, 'ab');
    return { ok: r.ok, engineState: r.state, notes: ['A/B: the base is golden/perf/ab-base.tsv'].concat(r.notes), results: r.results };
  },
});
