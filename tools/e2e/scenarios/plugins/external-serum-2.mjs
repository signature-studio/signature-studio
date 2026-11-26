// plugins/external-serum-2: Serum 2 (external, Serum2.vst3/Contents/x86_64-win/Serum2.vst3 (19236864 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/external-serum-2',
  covers: ['plugins/external/serum-2', 'plugins/window/serum-2', 'plugins/param/serum-2', 'plugins/audio/serum-2', 'plugins/stop/serum-2'],
  expectFail: '#1336',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Serum 2", kind: 'external' });
  },
});
