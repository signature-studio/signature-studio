// delivery/document-security-headers: the document is served with the Content-Security-Policy of app/shared/headers.ts, exactly (every directive: no origin outside 'self').
import { defineScenario } from '../../lib/scenario.mjs';
import { CSP, pageFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/document-security-headers',
  covers: ['delivery/header/csp'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const r = await pageFetch(page, '/');
    const got = r.headers['content-security-policy'] ?? null;
    return { ok: r.status === 200 && got === CSP, notes: [`status ${r.status}; Content-Security-Policy ${got === CSP ? 'as app/shared/headers.ts' : 'differs'}`], results: { status: r.status, matches: got === CSP } };
  },
});
