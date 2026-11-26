// page/headers-isolation: the page is cross-origin isolated, and its document carries the isolation headers of app/shared/headers.ts (COOP, COEP, CORP) with the values there.
import { defineScenario } from '../../lib/scenario.mjs';
import { ISOLATION_HEADERS } from '../../../../app/shared/headers.ts';

export default defineScenario({
  id: 'page/headers-isolation',
  covers: ['page/headers/isolation'],
  async run({ guest, page }) {
    const h = await page.evaluate(async () => {
      const r = await fetch('/', { cache: 'no-store' });
      return { coop: r.headers.get('cross-origin-opener-policy'), coep: r.headers.get('cross-origin-embedder-policy'), corp: r.headers.get('cross-origin-resource-policy'), isolated: crossOriginIsolated };
    });
    const want = ISOLATION_HEADERS;
    const ok = h.coop === want['Cross-Origin-Opener-Policy'] && h.coep === want['Cross-Origin-Embedder-Policy'] && h.corp === want['Cross-Origin-Resource-Policy'] && h.isolated === true;
    const state = (await guest.state())?.state ?? null;
    return { ok, engineState: state, notes: [`COOP ${h.coop}, COEP ${h.coep}, CORP ${h.corp}; crossOriginIsolated ${h.isolated}`], results: h };
  },
});
