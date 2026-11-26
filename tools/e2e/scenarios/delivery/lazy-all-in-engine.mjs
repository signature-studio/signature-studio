// delivery/lazy-all-in-engine: the page's own lazy downloads (started when the engine starts, lib: app/src/main.ts startLazy) have all arrived and verified: the page's
// results list every lazy file of the manifest with ok (the file is verified by SHA-256 and size before the engine reads it).
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle, lazyFeatureIds, sleep } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/lazy-all-in-engine',
  covers: lazyFeatureIds(),
  displays: ['4k-dpr1'],
  async run({ page }) {
    const want = bundle().lazy.length;
    const t0 = Date.now();
    let n = 0;
    while (Date.now() - t0 < 240000) {
      n = await page.evaluate(() => (window.__app ? window.__app.lazy.length : 0)).catch(() => 0);
      if (n >= want) break;
      await sleep(1000);
    }
    const rows = await page.evaluate(() => window.__app.lazy.map((r) => ({ name: r.name, ok: r.ok, message: r.message })));
    const failed = rows.filter((r) => !r.ok).map((r) => `${r.name}: ${r.message}`);
    const ok = rows.length === want && failed.length === 0;
    return { ok, notes: [`lazy results ${rows.length} of ${want} (after ${Math.round((Date.now() - t0) / 1000)} s): ok ${rows.filter((r) => r.ok).length}`], results: { results: rows.length, want, ok: rows.filter((r) => r.ok).length, failed: failed.slice(0, 20) } };
  },
});
