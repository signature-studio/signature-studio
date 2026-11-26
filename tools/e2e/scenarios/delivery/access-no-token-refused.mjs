// delivery/access-no-token-refused: without a token the Worker refuses the request on the wire (403): nothing of the bundle is served without the Access check.
import { defineScenario } from '../../lib/scenario.mjs';
import { wireFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/access-no-token-refused',
  covers: ['delivery/access/no-token-refused'],
  displays: ['4k-dpr1'],
  async run() {
    const r = await wireFetch('/assets/manifest.json', { token: null });
    const doc = await wireFetch('/', { token: null });
    const ok = r.status === 403 && doc.status === 403;
    return { ok, notes: [`no token: manifest ${r.status}, document ${doc.status}`], results: { manifest: r.status, document: doc.status } };
  },
});
