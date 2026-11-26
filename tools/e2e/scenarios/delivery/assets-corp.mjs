// delivery/assets-corp: an asset of the bundle is served with Cross-Origin-Resource-Policy: same-origin (the Worker's base headers, on /assets/ too).
import { defineScenario } from '../../lib/scenario.mjs';
import { ISOLATION, headerDiffs, pageFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/assets-corp',
  covers: ['delivery/header/corp'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const r = await pageFetch(page, '/assets/manifest.json', 'HEAD');
    const want = { 'Cross-Origin-Resource-Policy': ISOLATION['Cross-Origin-Resource-Policy'] };
    const diffs = headerDiffs(r.headers, want);
    return { ok: r.status === 200 && diffs.length === 0, notes: [`status ${r.status}; ${diffs.length === 0 ? 'CORP same-origin' : diffs.join('; ')}`], results: { status: r.status, diffs } };
  },
});
