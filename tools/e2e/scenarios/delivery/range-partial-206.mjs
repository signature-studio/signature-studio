// delivery/range-partial-206: a byte range of a stored object is answered 206 with Content-Range of the stored size and exactly the bytes asked (the engine reads its packs this way).
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle, workerFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/range-partial-206',
  covers: ['delivery/range/partial-206'],
  displays: ['4k-dpr1'],
  async run() {
    const ref = bundle().boot.find((b) => b.kind === 'seed').ref;
    const res = await workerFetch(ref.url, { headers: { Range: 'bytes=0-15' } });
    const buf = Buffer.from(await res.arrayBuffer());
    const cr = res.headers.get('content-range');
    const want = `bytes 0-15/${ref.encodedSize}`;
    const ok = res.status === 206 && cr === want && buf.length === 16;
    return { ok, notes: [`status ${res.status}; Content-Range ${cr} (want ${want}); ${buf.length} bytes`], results: { status: res.status, contentRange: cr, bytes: buf.length } };
  },
});
