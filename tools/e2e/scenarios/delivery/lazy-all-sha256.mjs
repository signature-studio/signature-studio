// delivery/lazy-all-sha256: every lazy file of the manifest (the files the engine reads after it has started) is fetched from the page: HTTP 200, and the bytes the
// page gets (decoded from brotli by the browser) have the manifest's size and SHA-256. Four fetches at a time, no cache. Each file is a feature of its own (covers).
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle, lazyFeatureIds } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/lazy-all-sha256',
  covers: lazyFeatureIds(),
  displays: ['4k-dpr1'],
  async run({ page }) {
    const items = bundle().lazy.map((l) => ({ name: l.name, url: l.ref.url, sha256: l.ref.sha256, size: l.ref.size }));
    const rows = await page.evaluate(async (list) => {
      const hex = (buf) => Array.from(new Uint8Array(buf), (b) => b.toString(16).padStart(2, '0')).join('');
      const out = new Array(list.length);
      let next = 0;
      const one = async (i) => {
        const it = list[i];
        try {
          const r = await fetch(it.url, { cache: 'no-store' });
          const buf = await r.arrayBuffer();
          const sha = hex(await crypto.subtle.digest('SHA-256', buf));
          out[i] = { name: it.name, status: r.status, size: buf.byteLength, shaOk: sha === it.sha256, sizeOk: buf.byteLength === it.size };
        } catch (e) {
          out[i] = { name: it.name, status: 0, size: 0, shaOk: false, sizeOk: false, error: String(e?.message ?? e).slice(0, 120) };
        }
      };
      const lane = async () => { while (next < list.length) { const i = next++; await one(i); } };
      await Promise.all([lane(), lane(), lane(), lane()]);
      return out;
    }, items);
    const bad = rows.filter((r) => !(r.status === 200 && r.shaOk && r.sizeOk));
    const names = bad.map((r) => r.name);
    const total = rows.reduce((a, r) => a + r.size, 0);
    const ok = rows.length === items.length && names.length === 0;
    return {
      ok,
      notes: [`lazy ${items.length}: HTTP 200 on ${rows.filter((r) => r.status === 200).length}, SHA-256 and size match on ${rows.filter((r) => r.shaOk && r.sizeOk).length}; ${total} bytes`],
      results: { lazy: items.length, status200: rows.filter((r) => r.status === 200).length, shaMatch: rows.filter((r) => r.shaOk && r.sizeOk).length, mismatches: names.slice(0, 50), bytes: total },
    };
  },
});
