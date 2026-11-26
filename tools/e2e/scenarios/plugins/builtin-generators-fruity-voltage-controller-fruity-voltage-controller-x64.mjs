// plugins/builtin-generators-fruity-voltage-controller-fruity-voltage-controller-x64: Fruity Voltage Controller (generators, DLL Fruity Voltage Controller_x64.dll (4087808 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fruity-voltage-controller-fruity-voltage-controller-x64',
  covers: ['plugins/generators/fruity-voltage-controller-fruity-voltage-controller-x64', 'plugins/window/generators-fruity-voltage-controller-fruity-voltage-controller-x64', 'plugins/param/generators-fruity-voltage-controller-fruity-voltage-controller-x64', 'plugins/audio/generators-fruity-voltage-controller-fruity-voltage-controller-x64', 'plugins/stop/generators-fruity-voltage-controller-fruity-voltage-controller-x64'],
  expectFail: '#1470',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Voltage Controller", kind: 'generators' });
  },
});
