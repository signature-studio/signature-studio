// perf/latency-click-to-pixel: from the click on the File label to the picture of the drop-down (the page's own latency records: the input's DOM time to the frame that answered it); 6 clicks, the median, judged against the baseline. The display is random at DPR too (the latency is the one measure that takes the random DPR).
import { defineScenario } from '../../lib/scenario.mjs';
import { LIMIT, Judge, baseOf, loadNow, menuLatency } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/latency-click-to-pixel',
  covers: ['perf/latency/click-to-pixel'],
  displays: ['4k-dpr1', '4k-random'],
  async run(ctx) {
    const j = new Judge(baseOf('base'));
    const r = await menuLatency(ctx.guest, 'File', 6);
    const ok = r.median !== null && j.ratio('latency.click.ms', r.median, { kind: 'upper', limit: LIMIT.latency, unit: 'ms', samples: r.values });
    j.notes.push(`${r.values.length} clicks; answered ${r.values.filter((x) => x !== null).length}; values ${r.values.map((x) => (x === null ? 'none' : Math.round(x))).join(', ')} ms; load ${loadNow().join(' ')}`);
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
