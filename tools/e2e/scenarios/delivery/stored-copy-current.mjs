// delivery/stored-copy-current: the page's stored copies (OPFS fl/cache/assets, one file per SHA-256) hold only the bundle's own versions: every entry is named by a SHA-256 the
// manifest names, so an older version's copy is not served (the page removes the copies no manifest names: src/bundle/assetstore.ts clean).
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle, bundleShas } from '../../lib/delivery-tests.mjs';

const ENTRY = /^([0-9a-f]{64})\.[a-z0-9]{1,8}$/;

export default defineScenario({
  id: 'delivery/stored-copy-current',
  covers: ['delivery/cache/stored-copy-current'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const listing = await page.evaluate(async () => {
      try {
        let d = await navigator.storage.getDirectory();
        for (const n of ['fl', 'cache', 'assets']) d = await d.getDirectoryHandle(n);
        const names = [];
        for await (const [name, h] of d.entries()) names.push({ name, kind: h.kind });
        return { ok: true, names };
      } catch (e) {
        return { ok: false, error: String(e?.name ?? e), names: [] };
      }
    });
    const shas = bundleShas();
    const stale = listing.names.filter((e) => e.kind !== 'file' || !ENTRY.test(e.name) || !shas.has(e.name.slice(0, 64)));
    // one stored copy per distinct SHA-256 (two files of the bundle with the same bytes share one copy)
    const distinct = shas.size;
    const ok = listing.ok && listing.names.length > 0 && stale.length === 0 && listing.names.length <= distinct;
    return { ok, notes: [`stored copies ${listing.names.length} (the bundle has ${distinct} distinct SHA-256 values); names not of this bundle: ${stale.length}${listing.ok ? '' : `; OPFS: ${listing.error}`}`], results: { entries: listing.names.length, distinct, stale: stale.map((e) => e.name.slice(0, 16)), error: listing.ok ? null : listing.error } };
  },
});
