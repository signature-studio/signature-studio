// delivery/conditional-etag-304: a request that names the stored object's ETag in If-None-Match gets 304 with no body (a page that has the object does not download it again).
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle, workerFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/conditional-etag-304',
  covers: ['delivery/conditional/etag-304'],
  displays: ['4k-dpr1'],
  async run() {
    const ref = bundle().boot.find((b) => b.kind === 'glue').ref;
    const first = await workerFetch(ref.url, { method: 'HEAD' });
    const etag = first.headers.get('etag');
    const again = await workerFetch(ref.url, { method: 'HEAD', headers: { 'If-None-Match': etag ?? '' } });
    const ok = first.status === 200 && !!etag && again.status === 304;
    return { ok, notes: [`first ${first.status} with ETag ${etag ? 'set' : 'missing'}; with If-None-Match ${again.status}`], results: { first: first.status, again: again.status } };
  },
});
