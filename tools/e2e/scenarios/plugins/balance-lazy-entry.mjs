// plugins/balance-lazy-entry: the Fruity Balance effect is a lazy module of the bundle: its entry in the manifest has a size and a SHA-256 (plugins.tsv: the Balance row).
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'plugins/balance-lazy-entry',
  covers: ['plugins/effects/fruity-balance-fruity-balance-x64'],
  async run({ page, origin }) {
    const e = await page.evaluate(async (o) => {
      const m = await (await fetch(`${o}/assets/manifest.json`)).json();
      return (m.lazy ?? []).find((l) => /fruity_balance/i.test(l.path)) ?? null;
    }, origin);
    const ok = !!e && e.size > 0 && /^[0-9a-f]{64}$/.test(e.sha256);
    return { ok, notes: [e ? `${e.path}: ${e.size} B, sha256 ${String(e.sha256).slice(0, 12)}` : 'no Balance entry in the manifest'] };
  },
});
