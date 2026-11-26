// delivery/assets-accept-ranges-etag: each start-up asset of the bundle (HEAD in the page) says Accept-Ranges: bytes, an ETag (the MD5 of the stored object, quoted) and Vary: Accept-Encoding.
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle, pageFetch } from '../../lib/delivery-tests.mjs';

const ETAG = /^"[0-9a-f]{32}"$/;

export default defineScenario({
  id: 'delivery/assets-accept-ranges-etag',
  covers: ['delivery/header/accept-ranges', 'delivery/header/etag', 'delivery/header/vary'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const rows = [];
    for (const b of bundle().boot) {
      const r = await pageFetch(page, b.ref.url, 'HEAD');
      const h = r.headers;
      const bad = [];
      if (r.status !== 200) bad.push(`status ${r.status}`);
      if ((h['accept-ranges'] ?? '') !== 'bytes') bad.push(`Accept-Ranges ${h['accept-ranges']}`);
      if (!ETAG.test(h.etag ?? '')) bad.push(`ETag ${h.etag}`);
      if (!/accept-encoding/i.test(h.vary ?? '')) bad.push(`Vary ${h.vary}`);
      rows.push({ name: b.name, bad });
    }
    const bad = rows.filter((x) => x.bad.length > 0);
    return { ok: bad.length === 0, notes: [`${rows.length} boot assets: Accept-Ranges bytes, quoted ETag and Vary Accept-Encoding on ${rows.length - bad.length}`], results: { checked: rows.length, bad } };
  },
});
