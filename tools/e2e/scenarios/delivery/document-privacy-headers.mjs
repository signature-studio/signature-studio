// delivery/document-privacy-headers: the document says X-Robots-Tag noindex, nofollow; X-Content-Type-Options nosniff; Referrer-Policy no-referrer (app/shared/headers.ts PRIVACY_HEADERS).
import { defineScenario } from '../../lib/scenario.mjs';
import { ISOLATION, headerDiffs, pageFetch } from '../../lib/delivery-tests.mjs';

const NAMES = ['X-Robots-Tag', 'X-Content-Type-Options', 'Referrer-Policy'];

export default defineScenario({
  id: 'delivery/document-privacy-headers',
  covers: ['delivery/header/x-robots-tag', 'delivery/header/nosniff', 'delivery/header/referrer-policy'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const r = await pageFetch(page, '/');
    const want = Object.fromEntries(NAMES.map((n) => [n, ISOLATION[n]]));
    const diffs = headerDiffs(r.headers, want);
    return { ok: r.status === 200 && diffs.length === 0, notes: [`status ${r.status}; ${diffs.length === 0 ? 'the three values match' : diffs.join('; ')}`], results: { status: r.status, want, diffs } };
  },
});
