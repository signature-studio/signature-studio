// perf/render-min-fps-mouse-1000hz: the pointer moves at 1000 Hz for 8 s over the desktop; the least one-second window of the rAF counter, judged against the baseline.
import { defineScenario } from '../../lib/scenario.mjs';
import { measureFps, moveMouse } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/render-min-fps-mouse-1000hz',
  covers: ['perf/render/min-fps-mouse-1000hz'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const r = await measureFps(ctx, 'render.minFps.mouse1000', 8000, () => moveMouse(ctx.guest, { ms: 8000, hz: 1000 }));
    return { ok: r.ok, engineState: r.state, notes: ['the pointer: 1000 Hz (CDP mouseMoved)'].concat(r.notes), results: r.results };
  },
});
