// plugins/preset-sytrus-load: loads a Sytrus preset from the Plugin database and the preset changes a parameter. expectFail #1535: the entry cannot be chosen (no row is drawn under Installed; the classification data is not in the bundle), so no preset is reached (#1508).
import { defineScenario } from '../../lib/scenario.mjs';
import { insertFromDatabase } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/preset-sytrus-load',
  covers: ['plugins/preset/sytrus-load'],
  expectFail: '#1535',
  async run({ guest }) {
    const r = await insertFromDatabase(guest, 'Sytrus');
    return { ok: false, engineState: (await guest.state())?.state ?? null, notes: [...r.notes, 'no Sytrus preset is loaded: the entry cannot be chosen'], results: r.results };
  },
});
