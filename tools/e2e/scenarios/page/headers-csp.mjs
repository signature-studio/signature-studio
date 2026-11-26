// page/headers-csp: the document's Content-Security-Policy is the one of app/shared/headers.ts (every source is 'self' or the blob of the engine's glue, nothing else).
import { defineScenario } from '../../lib/scenario.mjs';
import { CSP } from '../../../../app/shared/headers.ts';

export default defineScenario({
  id: 'page/headers-csp',
  covers: ['page/headers/csp'],
  async run({ guest, page }) {
    const got = await page.evaluate(async () => (await fetch('/', { cache: 'no-store' })).headers.get('content-security-policy'));
    const state = (await guest.state())?.state ?? null;
    return { ok: got === CSP, engineState: state, notes: [`CSP ${got === CSP ? 'as app/shared/headers.ts' : `differs: ${String(got).slice(0, 120)}`}`], results: { same: got === CSP } };
  },
});
