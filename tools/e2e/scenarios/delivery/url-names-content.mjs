// delivery/url-names-content: every object's URL names its content (the first 12 hex digits of its SHA-256 before the extension), so a URL that is cached as immutable
// can only ever hold the bytes it names. Checked on the manifest's references (no request).
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/url-names-content',
  covers: ['delivery/cache/url-names-content'],
  displays: ['4k-dpr1'],
  async run() {
    const refs = bundle().all.map((r) => r.ref);
    const bad = refs.filter((r) => !new RegExp(`\\.${r.sha256.slice(0, 12)}\\.[a-z0-9]+$`).test(r.url)).map((r) => r.url);
    const urls = new Set(refs.map((r) => r.url));
    const dup = refs.length - urls.size;
    return { ok: bad.length === 0 && dup === 0, notes: [`${refs.length} URLs: the first 12 hex digits of the SHA-256 in ${refs.length - bad.length}; ${dup} repeated`], results: { checked: refs.length, bad: bad.slice(0, 10), badCount: bad.length, repeated: dup } };
  },
});
