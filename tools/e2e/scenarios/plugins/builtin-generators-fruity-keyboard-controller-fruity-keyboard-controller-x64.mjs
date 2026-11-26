// plugins/builtin-generators-fruity-keyboard-controller-fruity-keyboard-controller-x64: Fruity Keyboard Controller (generators, DLL Fruity Keyboard Controller_x64.dll (4017152 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fruity-keyboard-controller-fruity-keyboard-controller-x64',
  covers: ['plugins/generators/fruity-keyboard-controller-fruity-keyboard-controller-x64', 'plugins/window/generators-fruity-keyboard-controller-fruity-keyboard-controller-x64', 'plugins/param/generators-fruity-keyboard-controller-fruity-keyboard-controller-x64', 'plugins/audio/generators-fruity-keyboard-controller-fruity-keyboard-controller-x64', 'plugins/stop/generators-fruity-keyboard-controller-fruity-keyboard-controller-x64'],
  expectFail: '#1469',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Keyboard Controller", kind: 'generators' });
  },
});
