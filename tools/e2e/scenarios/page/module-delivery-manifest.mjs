// page/module-delivery-manifest: the bundle's manifest comes from the delivery: it is served (200) and names the files of the bundle (a non-empty object).
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'page/module-delivery-manifest',
  covers: ['page/module/delivery'],
  async run({ guest, page }) {
    const r = await page.evaluate(async () => { const res = await fetch('/assets/manifest.json', { cache: 'no-store' }); const j = await res.json().catch(() => null); return { status: res.status, keys: j && typeof j === 'object' ? Object.keys(j).length : 0 }; });
    const state = (await guest.state())?.state ?? null;
    return { ok: r.status === 200 && r.keys > 0, engineState: state, notes: [`manifest ${r.status} with ${r.keys} top-level keys`], results: r };
  },
});
