// delivery/assets-cache-manifest: both manifests (/assets/manifest.json and the bundle's /assets/fl-Oz/manifest.json) are served no-cache, so a new version is seen at once
// (the rule of app/shared/headers.ts cacheControlFor: a mutable name is no-cache; the Worker adds no-transform).
import { defineScenario } from '../../lib/scenario.mjs';
import { cacheMissing, pageFetch } from '../../lib/delivery-tests.mjs';

const PATHS = ['/assets/manifest.json', '/assets/fl-Oz/manifest.json'];

export default defineScenario({
  id: 'delivery/assets-cache-manifest',
  covers: ['delivery/cache/manifest-no-cache'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const rows = [];
    for (const p of PATHS) {
      const r = await pageFetch(page, p, 'HEAD');
      rows.push({ path: p, status: r.status, cacheControl: r.headers['cache-control'] ?? null, missing: r.status === 200 ? cacheMissing(r.headers['cache-control'], p) : ['status'] });
    }
    const ok = rows.every((x) => x.missing.length === 0 && x.cacheControl.includes('no-cache'));
    return { ok, notes: rows.map((x) => `${x.path}: ${x.status}, Cache-Control ${x.cacheControl}`), results: rows };
  },
});
