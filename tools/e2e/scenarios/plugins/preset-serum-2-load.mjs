// plugins/preset-serum-2-load: loads a Serum 2 preset from the Plugin database and the preset changes a parameter. expectFail #1324 (the external plugins): the entry cannot be chosen (no row is drawn under Installed, #1508), so no preset is reached.
import { defineScenario } from '../../lib/scenario.mjs';
import { insertFromDatabase } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/preset-serum-2-load',
  covers: ['plugins/preset/serum-2-load'],
  expectFail: '#1324',
  async run({ guest }) {
    const r = await insertFromDatabase(guest, 'Serum 2');
    return { ok: false, engineState: (await guest.state())?.state ?? null, notes: [...r.notes, 'no Serum 2 preset is loaded: the entry cannot be chosen'], results: r.results };
  },
});
