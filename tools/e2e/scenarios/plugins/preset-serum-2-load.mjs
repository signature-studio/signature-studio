// plugins/preset-serum-2-load: loads a Serum 2 preset from the Plugin database and the preset changes a parameter. expectFail #1336: the Plugin database has no entry for Serum 2 in this build (#1484) and the plugin's own lift is open (#1336).
import { defineScenario } from '../../lib/scenario.mjs';
import { databaseHas, databaseLines, openPluginDatabase } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/preset-serum-2-load',
  covers: ['plugins/preset/serum-2-load'],
  expectFail: '#1336',
  async run({ guest }) {
    const db = await openPluginDatabase(guest);
    if (!db.opened) return { ok: false, notes: [`the picker did not open (${db.error})`] };
    const lines = await databaseLines(guest);
    if (!databaseHas(lines, 'Serum 2')) return { ok: false, engineState: (await guest.state())?.state ?? null, notes: ['no Serum 2 preset: the Plugin database has no entry for Serum 2'] };
    return { ok: false, engineState: (await guest.state())?.state ?? null, notes: ['the preset entry is listed, but loading it is not reachable in this build'] };
  },
});
