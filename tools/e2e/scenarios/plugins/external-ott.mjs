// plugins/external-ott: OTT (external, OTT.vst3 (3244032 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/external-ott',
  covers: ['plugins/external/ott', 'plugins/window/ott', 'plugins/param/ott', 'plugins/audio/ott', 'plugins/stop/ott'],
  expectFail: '#1337',
  async run({ guest }) {
    return pluginSteps(guest, { name: "OTT", kind: 'external' });
  },
});
