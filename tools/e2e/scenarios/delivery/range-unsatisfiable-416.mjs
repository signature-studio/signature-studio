// delivery/range-unsatisfiable-416: a range that starts past the end of a stored object is answered 416 with Content-Range "bytes */<size>" and no body.
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle, workerFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/range-unsatisfiable-416',
  covers: ['delivery/range/unsatisfiable-416'],
  displays: ['4k-dpr1'],
  async run() {
    const ref = bundle().boot.find((b) => b.kind === 'seed').ref;
    const res = await workerFetch(ref.url, { headers: { Range: `bytes=${ref.encodedSize + 10}-` } });
    const cr = res.headers.get('content-range');
    const want = `bytes */${ref.encodedSize}`;
    const ok = res.status === 416 && cr === want;
    return { ok, notes: [`status ${res.status}; Content-Range ${cr} (want ${want})`], results: { status: res.status, contentRange: cr } };
  },
});
