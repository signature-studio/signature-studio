// delivery/document-coep: the document is served with Cross-Origin-Embedder-Policy: require-corp (what the engine's shared memory needs).
import { defineScenario } from '../../lib/scenario.mjs';
import { ISOLATION, pageFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/document-coep',
  covers: ['delivery/header/coep'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const r = await pageFetch(page, '/');
    const want = ISOLATION['Cross-Origin-Embedder-Policy'];
    const got = r.headers['cross-origin-embedder-policy'] ?? null;
    return { ok: r.status === 200 && got === want, notes: [`status ${r.status}; Cross-Origin-Embedder-Policy ${got} (want ${want})`], results: { status: r.status, want, got } };
  },
});
