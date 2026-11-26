// plugins/sytrus-lazy-entry: the Sytrus generator is a lazy module of the bundle: its entry in the manifest has a size and a SHA-256 (plugins.tsv: the Sytrus row).
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'plugins/sytrus-lazy-entry',
  covers: ['plugins/generators/sytrus-sytrus-x64'],
  async run({ page, origin }) {
    const e = await page.evaluate(async (o) => {
      const m = await (await fetch(`${o}/assets/manifest.json`)).json();
      return (m.lazy ?? []).find((l) => /sytrus/i.test(l.path)) ?? null;
    }, origin);
    const ok = !!e && e.size > 0 && /^[0-9a-f]{64}$/.test(e.sha256);
    return { ok, notes: [e ? `${e.path}: ${e.size} B, sha256 ${String(e.sha256).slice(0, 12)}` : 'no Sytrus entry in the manifest'] };
  },
});
