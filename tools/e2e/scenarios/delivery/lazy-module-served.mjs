// delivery/lazy-module-served: a lazy module of the manifest (its URL from the manifest) is served: 200, and the bytes the page gets (decoded from br) are the manifest's size.
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'delivery/lazy-module-served',
  covers: ['delivery/lazy/fruity-filter-x64-7000000-dll'],
  async run({ page, origin }) {
    const r = await page.evaluate(async (o) => {
      const m = await (await fetch(`${o}/assets/manifest.json`)).json();
      const l = m.lazy.find((x) => x.path === 'fruity_filter_x64.7000000.dll');
      if (!l) return { status: null, want: null, got: null };
      const res = await fetch(`${o}${l.url}`);
      const buf = await res.arrayBuffer();
      return { status: res.status, want: l.size, got: buf.byteLength };
    }, origin);
    return { ok: r.status === 200 && r.want === r.got, notes: [`status ${r.status}, bytes ${r.got} (manifest ${r.want})`] };
  },
});
