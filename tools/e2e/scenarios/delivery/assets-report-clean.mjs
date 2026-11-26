// delivery/assets-report-clean: the page's report of its four start-up assets has no failure: each came from the network, the HTTP cache or the stored copy, and none has a fail code.
import { defineScenario } from '../../lib/scenario.mjs';
import { bootSha8, bundle } from '../../lib/delivery-tests.mjs';

const ORIGINS = ['network', 'http-cache', 'storage-cache'];

export default defineScenario({
  id: 'delivery/assets-report-clean',
  covers: ['delivery/boot/assets-report-clean'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const rep = await page.evaluate(() => window.__app.assetsReport());
    const boot = (rep?.items ?? []).filter((i) => bootSha8().has(i.sha));
    const bad = boot.filter((i) => i.fail !== null || !ORIGINS.includes(i.origin)).map((i) => ({ kind: i.kind, origin: i.origin, fail: i.fail, store: i.store }));
    const ok = boot.length === bundle().boot.length && bad.length === 0;
    return { ok, notes: [`start-up assets reported ${boot.length} of ${bundle().boot.length}; with a failure or an unknown origin: ${bad.length}`], results: { reported: boot.length, bad } };
  },
});
