// delivery/pre-boot-bytes: the bytes the page takes before the engine starts are the bundle's start-up set: the four assets (engine wasm, seed pack, DLL pack, glue) are
// each read once, and their decoded bytes are the manifest's sizes (the ratio is 1). The run's own report (window.__app.assetsReport) is the measure.
import { defineScenario } from '../../lib/scenario.mjs';
import { bootSha8, bundle, sizeOf } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/pre-boot-bytes',
  covers: ['delivery/boot/pre-boot-bytes'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const rep = await page.evaluate(() => window.__app.assetsReport());
    const boot = (rep?.items ?? []).filter((i) => bootSha8().has(i.sha));
    const got = boot.reduce((a, i) => a + i.bytes, 0);
    const want = sizeOf(bundle().boot);
    const ratio = want > 0 ? Math.round((got / want) * 1e6) / 1e6 : null;
    const ok = boot.length === bundle().boot.length && ratio === 1;
    return { ok, notes: [`before the engine: ${boot.length} start-up assets, decoded bytes ${got} = ${ratio} x the manifest's ${want}`], results: { assets: boot.length, bytes: got, manifestBytes: want, ratio } };
  },
});
