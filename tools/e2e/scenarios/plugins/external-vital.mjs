// plugins/external-vital: Vital (external, Vital.vst3 (15571240 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/external-vital',
  covers: ['plugins/external/vital', 'plugins/window/vital', 'plugins/param/vital', 'plugins/audio/vital', 'plugins/stop/vital'],
  expectFail: '#1338',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Vital", kind: 'external' });
  },
});
