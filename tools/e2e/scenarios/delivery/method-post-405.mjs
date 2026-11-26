// delivery/method-post-405: a POST to an object of /assets/ is refused 405 with Allow: GET, HEAD (the delivery is read-only).
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle, workerFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/method-post-405',
  covers: ['delivery/method/post-405'],
  displays: ['4k-dpr1'],
  async run() {
    const ref = bundle().boot.find((b) => b.kind === 'glue').ref;
    const res = await workerFetch(ref.url, { method: 'POST', body: 'x' });
    const allow = res.headers.get('allow');
    const ok = res.status === 405 && allow === 'GET, HEAD';
    return { ok, notes: [`POST ${res.status}; Allow ${allow}`], results: { status: res.status, allow } };
  },
});
