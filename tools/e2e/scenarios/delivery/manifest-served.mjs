// delivery/manifest-served: the bundle's manifest is served from the page (200, JSON, schema 1) with every lazy file of the bundle that the manifest names.
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle, pageFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/manifest-served',
  covers: ['delivery/manifest/served-json'],
  displays: ['4k-dpr1'],
  async run({ page, origin }) {
    const r = await page.evaluate(async (o) => {
      const res = await fetch(`${o}/assets/manifest.json`, { cache: 'no-store' });
      const text = await res.text();
      let j = null;
      try { j = JSON.parse(text); } catch { /* not JSON */ }
      return { status: res.status, type: res.headers.get('content-type'), schema: j?.schema ?? null, lazy: j?.lazy?.length ?? 0 };
    }, origin);
    const want = bundle().lazy.length;
    const ok = r.status === 200 && /application\/json/.test(r.type ?? '') && r.schema === 1 && r.lazy === want;
    return { ok, notes: [`status ${r.status}, type ${r.type}, schema ${r.schema}, lazy ${r.lazy} (the bundle: ${want})`], results: { status: r.status, lazy: r.lazy, want } };
  },
});
