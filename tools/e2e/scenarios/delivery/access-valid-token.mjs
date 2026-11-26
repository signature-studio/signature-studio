// delivery/access-valid-token: with a token the Access check accepts (signed by the run's key, the run's team and AUD), the Worker serves the manifest: 200 on the wire.
import { defineScenario } from '../../lib/scenario.mjs';
import { wireFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/access-valid-token',
  covers: ['delivery/access/valid-token-passes'],
  displays: ['4k-dpr1'],
  async run() {
    const r = await wireFetch('/assets/manifest.json');
    const ok = r.status === 200;
    return { ok, notes: [`with a valid token: ${r.status}`], results: { status: r.status } };
  },
});
