// delivery/assets-no-transform: every object of /assets/ carries no-transform (app/worker/index.ts adds it to the stored Cache-Control), so nothing between the Worker and the browser re-encodes the stored bytes.
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle, directivesOf, pageFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/assets-no-transform',
  covers: ['delivery/cache/no-transform'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const urls = [...bundle().all.map((r) => r.ref.url), '/assets/manifest.json', '/assets/fl-Oz/manifest.json'];
    const missing = [];
    for (const u of urls) {
      const r = await pageFetch(page, u, 'HEAD');
      if (r.status !== 200 || !directivesOf(r.headers['cache-control']).has('no-transform')) missing.push({ status: r.status, cacheControl: r.headers['cache-control'] ?? null });
    }
    return { ok: missing.length === 0, notes: [`${urls.length} objects under /assets/: no-transform on ${urls.length - missing.length}`], results: { checked: urls.length, missing: missing.slice(0, 10), missingCount: missing.length } };
  },
});
