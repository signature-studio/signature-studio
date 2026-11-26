// page/headers-manifest-cache: the bundle's manifest is served with the cache rule of the Worker (cacheControlFor: the manifest may change, so it is not kept as immutable), and it parses.
import { defineScenario } from '../../lib/scenario.mjs';
import { cacheControlFor } from '../../../../app/shared/headers.ts';

export default defineScenario({
  id: 'page/headers-manifest-cache',
  covers: ['page/headers/manifest-cache'],
  async run({ guest, page }) {
    const r = await page.evaluate(async () => { const res = await fetch('/assets/manifest.json', { cache: 'no-store' }); const text = await res.text(); let parsed = null; try { parsed = JSON.parse(text); } catch { /* not JSON */ } return { status: res.status, cc: res.headers.get('cache-control'), json: parsed !== null && typeof parsed === 'object' }; });
    const want = cacheControlFor('/assets/manifest.json');
    const state = (await guest.state())?.state ?? null;
    const ok = r.status === 200 && r.json === true && typeof r.cc === 'string' && r.cc.includes(want);
    return { ok, engineState: state, notes: [`manifest ${r.status}, Cache-Control "${r.cc}" (the rule says "${want}"), JSON ${r.json}`], results: { status: r.status, cc: r.cc, json: r.json, want } };
  },
});
