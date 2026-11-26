// perf/latency-key-to-pixel: with the File drop-down open, ArrowDown is pressed 10 times; the input-to-picture latency of each press, the median judged against the baseline.
import { defineScenario } from '../../lib/scenario.mjs';
import { Judge, LIMIT, baseOf, keyLatency } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/latency-key-to-pixel',
  covers: ['perf/latency/key-to-pixel'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const j = new Judge(baseOf('base'));
    const r = await keyLatency(ctx.guest, 10);
    const ok = r.median !== null && j.ratio('latency.key.ms', r.median, { kind: 'upper', limit: LIMIT.latency, unit: 'ms', samples: r.values });
    j.notes.push(`${r.values.filter((x) => x !== null).length} of ${r.values.length} presses answered; values ${r.values.map((x) => (x === null ? 'none' : Math.round(x))).join(', ')} ms`);
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
