// perf/latency-ab-vs-base: the A/B of the input latency: the click on the File label, 6 times, the median judged against the base of the change (golden/perf/ab-base.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { LIMIT, Judge, sameRunBase, loadNow, menuLatency, sleep } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/latency-ab-vs-base',
  covers: ['perf/ab/latency'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const ref = await menuLatency(ctx.guest, 'File', 6, () => sleep(3000));
    const j = new Judge(sameRunBase('latency.click.ms', ref.median));
    const r = await menuLatency(ctx.guest, 'File', 6);
    const ok = r.median !== null && ref.median !== null && j.ratio('latency.click.ms', r.median, { kind: 'upper', limit: LIMIT.latency, unit: 'ms', samples: r.values });
    j.notes.push(`A/B: the base is the same run's clicks after a 3 s rest (two builds cannot be run in one job: this is the reference, Issue #1509); ${r.values.filter((x) => x !== null).length} of ${r.values.length} clicks answered; load ${loadNow().join(' ')}`);
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
