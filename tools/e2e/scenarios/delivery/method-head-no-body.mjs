// delivery/method-head-no-body: HEAD on a stored object gives its headers (Content-Length of the stored size) and no body.
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle, workerFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/method-head-no-body',
  covers: ['delivery/method/head-no-body'],
  displays: ['4k-dpr1'],
  async run() {
    const ref = bundle().boot.find((b) => b.kind === 'dll').ref;
    const res = await workerFetch(ref.url, { method: 'HEAD' });
    const len = res.headers.get('content-length');
    const body = res.body === null ? null : Buffer.from(await res.arrayBuffer());
    const ok = res.status === 200 && len === String(ref.encodedSize) && (body === null || body.length === 0);
    return { ok, notes: [`HEAD ${res.status}; Content-Length ${len} (encodedSize ${ref.encodedSize}); body ${body === null ? 'none' : `${body.length} B`}`], results: { status: res.status, contentLength: len, body: body === null ? null : body.length } };
  },
});
