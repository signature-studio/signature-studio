// perf/latency-menu-open-edit: from the click on the Edit label to the picture of its drop-down; 5 clicks, the median, judged against the reference of the same run (the base is the same run: the same clicks after a 3 s rest (the reference), not golden/perf/*.tsv (Issue #1509)).
import { defineScenario } from '../../lib/scenario.mjs';
import { LIMIT, Judge, sameRunBase, loadNow, menuLatency, sleep } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/latency-menu-open-edit',
  covers: ['perf/latency/menu-open-edit'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const ref = await menuLatency(ctx.guest, 'Edit', 5, () => sleep(3000));
    const j = new Judge(sameRunBase('latency.menuOpen.edit.ms', ref.median));
    const r = await menuLatency(ctx.guest, 'Edit', 5);
    const ok = r.median !== null && ref.median !== null && j.ratio('latency.menuOpen.edit.ms', r.median, { kind: 'upper', limit: LIMIT.latency, unit: 'ms', samples: r.values });
    j.notes.push(`${r.values.filter((x) => x !== null).length} of ${r.values.length} clicks answered; values ${r.values.map((x) => (x === null ? 'none' : Math.round(x))).join(', ')} ms; load ${loadNow().join(' ')}`);
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
