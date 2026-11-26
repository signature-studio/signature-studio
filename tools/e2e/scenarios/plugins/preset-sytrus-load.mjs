// plugins/preset-sytrus-load: loads a Sytrus preset from the Plugin database and the preset changes a parameter. expectFail #1400: the Plugin database has no entry for Sytrus in this build (#1484) and the plugin's own lift is open (#1400).
import { defineScenario } from '../../lib/scenario.mjs';
import { databaseHas, databaseLines, openPluginDatabase } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/preset-sytrus-load',
  covers: ['plugins/preset/sytrus-load'],
  expectFail: '#1400',
  async run({ guest }) {
    const db = await openPluginDatabase(guest);
    if (!db.opened) return { ok: false, notes: [`the picker did not open (${db.error})`] };
    const lines = await databaseLines(guest);
    if (!databaseHas(lines, 'Sytrus')) return { ok: false, engineState: (await guest.state())?.state ?? null, notes: ['no Sytrus preset: the Plugin database has no entry for Sytrus'] };
    return { ok: false, engineState: (await guest.state())?.state ?? null, notes: ['the preset entry is listed, but loading it is not reachable in this build'] };
  },
});
