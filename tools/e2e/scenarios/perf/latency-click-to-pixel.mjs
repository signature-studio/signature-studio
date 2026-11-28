// perf/latency-click-to-pixel: from the click on the File label to the picture of the drop-down (the page's own latency records: the input's DOM time to the frame that answered it); 6 clicks, the median, judged against the reference of the same run (the base is the same run: the same clicks after a 3 s rest (the reference), not golden/perf/*.tsv (Issue #1509)). The display is random at DPR too (the latency is the one measure that takes the random DPR).
import { defineScenario } from '../../lib/scenario.mjs';
import { LIMIT, Judge, sameRunBase, loadNow, menuLatency, sleep } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/latency-click-to-pixel',
  covers: ['perf/latency/click-to-pixel'],
  displays: ['4k-dpr1', '4k-random'],
  async run(ctx) {
    const ref = await menuLatency(ctx.guest, 'File', 6, () => sleep(3000));
    const j = new Judge(sameRunBase('latency.click.ms', ref.median));
    const r = await menuLatency(ctx.guest, 'File', 6);
    const ok = r.median !== null && ref.median !== null && j.ratio('latency.click.ms', r.median, { kind: 'upper', limit: LIMIT.latency, unit: 'ms', samples: r.values });
    j.notes.push(`${r.values.length} clicks; answered ${r.values.filter((x) => x !== null).length}; values ${r.values.map((x) => (x === null ? 'none' : Math.round(x))).join(', ')} ms; load ${loadNow().join(' ')}`);
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
