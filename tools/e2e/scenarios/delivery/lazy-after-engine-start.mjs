// delivery/lazy-after-engine-start: the lazy downloads start after the engine has started (the timeline's lazy_start is not before engine_start): nothing the start-up needs waits for them.
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'delivery/lazy-after-engine-start',
  covers: ['delivery/boot/lazy-after-engine'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const tl = await page.evaluate(() => (window.__app ? window.__app.timeline : {}));
    const es = tl.engine_start, ls = tl.lazy_start;
    const ok = typeof es === 'number' && typeof ls === 'number' && ls >= es;
    return { ok, notes: [`engine_start ${es} ms, lazy_start ${ls} ms (page clock)`], results: { engineStart: es ?? null, lazyStart: ls ?? null } };
  },
});
