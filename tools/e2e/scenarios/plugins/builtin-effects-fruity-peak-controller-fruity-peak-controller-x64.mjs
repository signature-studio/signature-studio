// plugins/builtin-effects-fruity-peak-controller-fruity-peak-controller-x64: Fruity Peak Controller (effects, DLL Fruity Peak Controller_x64.dll (4044288 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-peak-controller-fruity-peak-controller-x64',
  covers: ['plugins/effects/fruity-peak-controller-fruity-peak-controller-x64', 'plugins/window/effects-fruity-peak-controller-fruity-peak-controller-x64', 'plugins/param/effects-fruity-peak-controller-fruity-peak-controller-x64', 'plugins/audio/effects-fruity-peak-controller-fruity-peak-controller-x64', 'plugins/stop/effects-fruity-peak-controller-fruity-peak-controller-x64'],
  expectFail: '#1446',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Peak Controller", kind: 'effects' });
  },
});
