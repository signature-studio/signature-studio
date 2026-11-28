// perf/cpu-idle-busy-60s: with no input for 60 s, the engine's busyPct (the sum of its idle reports' busy times over the wall time); judged against the same run's quiet reference (the first 20 s with no input; at most 1.5 times; the reference is the same run, the same measure with no sound (20 s), not golden/perf/*.tsv (Issue #1509)).
import { defineScenario } from '../../lib/scenario.mjs';
import { Judge, LIMIT, sameRunBase, busyPctOver, loadNow } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/cpu-idle-busy-60s',
  covers: ['perf/cpu/idle-busy'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const quiet = await busyPctOver(ctx.page, 20000);
    const j = new Judge(sameRunBase('cpu.idle.busyPct', quiet.pct));
    const b = await busyPctOver(ctx.page, 60000);
    const ok = b.pct !== null && j.ratio('cpu.idle.busyPct', b.pct, { kind: 'upper', limit: LIMIT.busy, unit: '%' });
    j.notes.push(`${b.idles} idle reports in ${b.seconds} s; load ${loadNow().join(' ')}`);
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
