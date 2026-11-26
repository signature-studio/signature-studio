// perf/render-min-fps-mouse: the pointer moves at 200 Hz for 8 s over the desktop; the least one-second window of the rAF counter, judged against the baseline.
import { defineScenario } from '../../lib/scenario.mjs';
import { measureFps, moveMouse } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/render-min-fps-mouse',
  covers: ['perf/render/min-fps-mouse'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const r = await measureFps(ctx, 'render.minFps.mouse', 8000, () => moveMouse(ctx.guest, { ms: 8000, hz: 200 }));
    return { ok: r.ok, engineState: r.state, notes: ['the pointer: 200 Hz (CDP mouseMoved)'].concat(r.notes), results: r.results };
  },
});
