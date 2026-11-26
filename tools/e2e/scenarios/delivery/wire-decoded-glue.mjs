// delivery/wire-decoded-glue: the page's own fetch of the engine glue (brotli on the wire, decoded by the browser) gives the manifest's bytes: the SHA-256 and the size match.
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/wire-decoded-glue',
  covers: ['delivery/compression/wire-decoded-glue'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const glue = bundle().boot.find((b) => b.kind === 'glue').ref;
    const r = await page.evaluate(async (url) => {
      const res = await fetch(url, { cache: 'no-store' });
      const buf = await res.arrayBuffer();
      const d = await crypto.subtle.digest('SHA-256', buf);
      return { status: res.status, size: buf.byteLength, sha256: Array.from(new Uint8Array(d), (b) => b.toString(16).padStart(2, '0')).join('') };
    }, glue.url);
    const ok = r.status === 200 && r.sha256 === glue.sha256 && r.size === glue.size;
    return { ok, notes: [`engine glue: status ${r.status}, ${r.size} B, SHA-256 ${r.sha256 === glue.sha256 ? 'as the manifest' : 'differs'}`], results: { status: r.status, size: r.size, matches: r.sha256 === glue.sha256 } };
  },
});
