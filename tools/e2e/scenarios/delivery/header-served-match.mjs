// delivery/header-served-match: the delivery's responses carry the values of app/shared/headers.ts (lib/served.mjs: the wire of wrangler dev --local, and the Worker
// read directly), and the Access gate refuses a request without a token (the same probe the run's row reports as servedHeadersMatch and accessGate).
import { defineScenario } from '../../lib/scenario.mjs';
import { probeServed } from '../../lib/served.mjs';
import { RAW_CACHE } from '../../lib/delivery.mjs';
import { BUNDLE, bundle, keyOf, live } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/header-served-match',
  covers: ['delivery/header/served-match'],
  displays: ['4k-dpr1'],
  async run() {
    const d = live();
    const b = bundle();
    const brFile = keyOf(b.boot[0].ref).slice(BUNDLE.length + 1);
    const r = await probeServed({ origin: d.upstream, token: d.newToken(), bundle: BUNDLE, brFile, objects: b.objects, accessEnv: d.accessEnv, manifestCacheStored: RAW_CACHE });
    const diffs = r.checks.flatMap((c) => c.diffs.map((x) => `${c.reading} ${c.path}: ${x}`));
    const ok = r.servedHeadersMatch === true && r.accessGate === true;
    return { ok, notes: [`servedHeadersMatch ${r.servedHeadersMatch}; the Access gate refuses a request without a token: ${r.accessGate}`], results: { servedHeadersMatch: r.servedHeadersMatch, accessGate: r.accessGate, checks: r.checks.map((c) => ({ reading: c.reading, path: c.path, status: c.status, diffs: c.diffs })), diffs } };
  },
});
