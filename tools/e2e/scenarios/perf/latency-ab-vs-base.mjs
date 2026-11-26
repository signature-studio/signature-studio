// perf/latency-ab-vs-base: the A/B of the input latency: the click on the File label, 6 times, the median judged against the base of the change (golden/perf/ab-base.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { LIMIT, Judge, baseOf, loadNow, menuLatency } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/latency-ab-vs-base',
  covers: ['perf/ab/latency'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const j = new Judge(baseOf('ab'));
    const r = await menuLatency(ctx.guest, 'File', 6);
    const ok = r.median !== null && j.ratio('latency.click.ms', r.median, { kind: 'upper', limit: LIMIT.latency, unit: 'ms', samples: r.values });
    j.notes.push(`A/B: the base is golden/perf/ab-base.tsv; ${r.values.filter((x) => x !== null).length} of ${r.values.length} clicks answered; load ${loadNow().join(' ')}`);
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
