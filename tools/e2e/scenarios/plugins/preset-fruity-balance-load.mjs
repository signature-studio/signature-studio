// plugins/preset-fruity-balance-load: loads a Fruity Balance preset from the Plugin database and the preset changes a parameter. expectFail #1535: the entry cannot be chosen (no row is drawn under Installed; the classification data is not in the bundle), so no preset is reached (#1508).
import { defineScenario } from '../../lib/scenario.mjs';
import { insertFromDatabase } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/preset-fruity-balance-load',
  covers: ['plugins/preset/fruity-balance-load'],
  expectFail: '#1535',
  async run({ guest }) {
    const r = await insertFromDatabase(guest, 'Fruity Balance');
    return { ok: false, engineState: (await guest.state())?.state ?? null, notes: [...r.notes, 'no Fruity Balance preset is loaded: the entry cannot be chosen'], results: r.results };
  },
});
