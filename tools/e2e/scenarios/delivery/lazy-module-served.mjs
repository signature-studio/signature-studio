// delivery/lazy-module-served: a lazy module of the manifest (its URL from the manifest) is served: 200, and the bytes the page gets (decoded from br) are the manifest's size.
// The Fruity Filter DLL is found by its plugin name (effects.fruity-filter.fruity_filter_x64.<image base>.dll: the base is the one the build gave, Issue #1532).
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'delivery/lazy-module-served',
  covers: ['delivery/lazy/fruity-filter-x64-7000000-dll'],
  displays: ['4k-dpr1'],
  async run({ page, origin }) {
    const r = await page.evaluate(async (o) => {
      const m = await (await fetch(`${o}/assets/manifest.json`)).json();
      const l = m.lazy.find((x) => /^effects\.fruity-filter\.fruity_filter_x64\.[0-9a-f]+\.dll$/.test(x.path));
      if (!l) return { path: null, status: null, want: null, got: null };
      const res = await fetch(`${o}${l.url}`);
      const buf = await res.arrayBuffer();
      return { path: l.path, status: res.status, want: l.size, got: buf.byteLength };
    }, origin);
    return { ok: r.status === 200 && r.want === r.got, notes: [`path ${r.path}: status ${r.status}, bytes ${r.got} (manifest ${r.want})`] };
  },
});
