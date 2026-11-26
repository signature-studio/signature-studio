// plugins/manifest-lists-plugins: the bundle's manifest lists the plugin modules (lazy) by their file names: the Fruity Balance and the Sytrus DLLs.
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'plugins/manifest-lists-plugins',
  covers: ['plugins/effects/fruity-balance-fruity-balance-x64', 'plugins/generators/sytrus-sytrus-x64'],
  async run({ page, origin }) {
    const m = await page.evaluate(async (o) => (await fetch(`${o}/assets/manifest.json`)).json(), origin);
    const names = (m.lazy ?? []).map((l) => l.path);
    const ok = names.some((n) => /fruity_balance/i.test(n)) && names.some((n) => /sytrus/i.test(n));
    return { ok, notes: [`${names.length} lazy modules in the manifest`] };
  },
});
