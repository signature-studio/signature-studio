// page/headers-privacy: the document asks no search engine to index it, sniffs no type, and sends no referrer (X-Robots-Tag, X-Content-Type-Options, Referrer-Policy as in app/shared/headers.ts).
import { defineScenario } from '../../lib/scenario.mjs';
import { PRIVACY_HEADERS } from '../../../../app/shared/headers.ts';

export default defineScenario({
  id: 'page/headers-privacy',
  covers: ['page/headers/privacy'],
  async run({ guest, page }) {
    const h = await page.evaluate(async () => { const r = await fetch('/', { cache: 'no-store' }); return { robots: r.headers.get('x-robots-tag'), nosniff: r.headers.get('x-content-type-options'), referrer: r.headers.get('referrer-policy') }; });
    const want = PRIVACY_HEADERS;
    const ok = h.robots === want['X-Robots-Tag'] && h.nosniff === want['X-Content-Type-Options'] && h.referrer === want['Referrer-Policy'];
    const state = (await guest.state())?.state ?? null;
    return { ok, engineState: state, notes: [`X-Robots-Tag ${h.robots}; X-Content-Type-Options ${h.nosniff}; Referrer-Policy ${h.referrer}`], results: h };
  },
});
