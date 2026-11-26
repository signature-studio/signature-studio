// delivery/assets-cache-immutable: every object of the bundle (the boot assets and the lazy files) is served with the directives of its rule (cacheControlFor:
// the name carries the first 12 hex digits of its SHA-256, so the rule is immutable and max-age=31536000). Checked in the page for each object (HEAD).
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle, cacheMissing, pageFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/assets-cache-immutable',
  covers: ['delivery/cache/immutable-hashed'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const refs = bundle().all.map((r) => r.ref);
    const bad = [];
    for (const ref of refs) {
      const r = await pageFetch(page, ref.url, 'HEAD');
      const missing = r.status === 200 ? cacheMissing(r.headers['cache-control'], ref.url) : ['status'];
      if (missing.length > 0) bad.push({ url: ref.url.slice(-40), status: r.status, cacheControl: r.headers['cache-control'] ?? null, missing });
    }
    return { ok: bad.length === 0, notes: [`${refs.length} objects: the rule's directives served on ${refs.length - bad.length}, immutable with max-age=31536000 for the hashed names`], results: { checked: refs.length, bad: bad.slice(0, 20), badCount: bad.length } };
  },
});
