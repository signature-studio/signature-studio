// delivery/document-coop: the document is served with Cross-Origin-Opener-Policy: same-origin (the page is cross-origin isolated with COEP).
import { defineScenario } from '../../lib/scenario.mjs';
import { ISOLATION, pageFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/document-coop',
  covers: ['delivery/header/coop'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const r = await pageFetch(page, '/');
    const want = ISOLATION['Cross-Origin-Opener-Policy'];
    const got = r.headers['cross-origin-opener-policy'] ?? null;
    return { ok: r.status === 200 && got === want, notes: [`status ${r.status}; Cross-Origin-Opener-Policy ${got} (want ${want})`], results: { status: r.status, want, got } };
  },
});
