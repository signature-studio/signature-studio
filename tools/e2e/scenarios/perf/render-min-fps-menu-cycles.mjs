// perf/render-min-fps-menu-cycles: the File drop-down is opened and closed 10 times while the rAF counter runs; the least one-second window, judged against the baseline.
import { defineScenario } from '../../lib/scenario.mjs';
import { measureFps, openAndClose } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/render-min-fps-menu-cycles',
  covers: ['perf/render/min-fps-menu'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const cycles = async () => { for (let i = 0; i < 10; i++) await openAndClose(ctx.guest, 'File'); };
    const r = await measureFps(ctx, 'render.minFps.menu', 1000, cycles);
    return { ok: r.ok, engineState: r.state, notes: ['10 open/close cycles of File; the window is the cycles'].concat(r.notes), results: r.results };
  },
});
