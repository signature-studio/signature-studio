// perf/render-min-fps-menu-cycles: the File drop-down is opened and closed 10 times while the rAF counter runs; the least one-second window, judged against the baseline.
import { defineScenario } from '../../lib/scenario.mjs';
import { measureFps, openAndClose } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/render-min-fps-menu-cycles',
  covers: ['perf/render/min-fps-menu'],
  displays: ['4k-dpr1'],
  expectFail: 'new', // run of 2026-11-28: the least 1 s window 1.79 rAF/s during 10 File open/close cycles against 57.2 rAF/s with no input in the same run (the golden baseline of 2026-11 also had 1.95); a candidate for PM triage, not confirmed
  async run(ctx) {
    const cycles = async () => { for (let i = 0; i < 10; i++) await openAndClose(ctx.guest, 'File'); };
    const r = await measureFps(ctx, 'render.minFps.menu', 1000, cycles);
    return { ok: r.ok, engineState: r.state, notes: ['10 open/close cycles of File; the window is the cycles'].concat(r.notes), results: r.results };
  },
});
