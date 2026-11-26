// plugins/preset-fruity-balance-load: loads a Fruity Balance preset from the Plugin database and the preset changes a parameter. expectFail #1315: the Plugin database has no entry for Fruity Balance in this build (#1484) and the plugin's own lift is open (#1315).
import { defineScenario } from '../../lib/scenario.mjs';
import { databaseHas, databaseLines, openPluginDatabase } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/preset-fruity-balance-load',
  covers: ['plugins/preset/fruity-balance-load'],
  expectFail: '#1315',
  async run({ guest }) {
    const db = await openPluginDatabase(guest);
    if (!db.opened) return { ok: false, notes: [`the picker did not open (${db.error})`] };
    const lines = await databaseLines(guest);
    if (!databaseHas(lines, 'Fruity Balance')) return { ok: false, engineState: (await guest.state())?.state ?? null, notes: ['no Fruity Balance preset: the Plugin database has no entry for Fruity Balance'] };
    return { ok: false, engineState: (await guest.state())?.state ?? null, notes: ['the preset entry is listed, but loading it is not reachable in this build'] };
  },
});
