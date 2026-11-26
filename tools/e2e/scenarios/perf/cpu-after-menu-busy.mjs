// perf/cpu-after-menu-busy: the File drop-down is opened and closed 10 times, then the engine's busyPct over 30 s with no input; judged against the baseline (at most 1.5 times).
import { defineScenario } from '../../lib/scenario.mjs';
import { Judge, LIMIT, baseOf, busyPctOver, openAndClose } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/cpu-after-menu-busy',
  covers: ['perf/cpu/after-menu-busy'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const j = new Judge(baseOf('base'));
    for (let i = 0; i < 10; i++) await openAndClose(ctx.guest, 'File');
    const b = await busyPctOver(ctx.page, 30000);
    const ok = b.pct !== null && j.ratio('cpu.afterMenu.busyPct', b.pct, { kind: 'upper', limit: LIMIT.busy, unit: '%' });
    j.notes.push(`after 10 File open/close: ${b.idles} idle reports in ${b.seconds} s`);
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
