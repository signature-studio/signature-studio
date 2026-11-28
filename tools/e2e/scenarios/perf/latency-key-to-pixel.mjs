// perf/latency-key-to-pixel: with the File drop-down open, ArrowDown is pressed 10 times; the input-to-picture latency of each press, the median judged against the reference of the same run (the base is the same run: the same clicks after a 3 s rest (the reference), not golden/perf/*.tsv (Issue #1509)).
import { defineScenario } from '../../lib/scenario.mjs';
import { Judge, LIMIT, sameRunBase, keyLatency, sleep } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/latency-key-to-pixel',
  covers: ['perf/latency/key-to-pixel'],
  displays: ['4k-dpr1'],
  expectFail: 'new', // run of 2026-11-28: the median 15.97 ms with the presses 150 ms apart against 1.56 ms after a 3 s rest in the same run (ratio 10.2 > 1.5), 9 of 10 answered; a candidate for PM triage, not confirmed
  async run(ctx) {
    const ref = await keyLatency(ctx.guest, 10, () => sleep(3000));
    const j = new Judge(sameRunBase('latency.key.ms', ref.median));
    const r = await keyLatency(ctx.guest, 10);
    const ok = r.median !== null && ref.median !== null && j.ratio('latency.key.ms', r.median, { kind: 'upper', limit: LIMIT.latency, unit: 'ms', samples: r.values });
    j.notes.push(`${r.values.filter((x) => x !== null).length} of ${r.values.length} presses answered; values ${r.values.map((x) => (x === null ? 'none' : Math.round(x))).join(', ')} ms`);
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
