// delivery/manifest-served: the bundle's manifest is served (200, JSON) with the lazy list of the bundle (234 entries at the time of the inventory).
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'delivery/manifest-served',
  covers: ['delivery/lazy/fruity-filter-x64-7000000-dll'],
  async run({ page, origin }) {
    const r = await page.evaluate(async (o) => {
      const res = await fetch(`${o}/assets/manifest.json`);
      const j = res.ok ? await res.json() : null;
      return { status: res.status, type: res.headers.get('content-type'), lazy: j?.lazy?.length ?? 0 };
    }, origin);
    return { ok: r.status === 200 && r.lazy > 0, notes: [`status ${r.status}, type ${r.type}, lazy ${r.lazy}`] };
  },
});
