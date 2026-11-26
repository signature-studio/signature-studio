// plugins/builtin-generators-fruity-envelope-controller-fruity-envelope-controller-x64: Fruity Envelope Controller (generators, DLL Fruity Envelope Controller_x64.dll (6251520 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fruity-envelope-controller-fruity-envelope-controller-x64',
  covers: ['plugins/generators/fruity-envelope-controller-fruity-envelope-controller-x64', 'plugins/window/generators-fruity-envelope-controller-fruity-envelope-controller-x64', 'plugins/param/generators-fruity-envelope-controller-fruity-envelope-controller-x64', 'plugins/audio/generators-fruity-envelope-controller-fruity-envelope-controller-x64', 'plugins/stop/generators-fruity-envelope-controller-fruity-envelope-controller-x64'],
  expectFail: '#1469',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Envelope Controller", kind: 'generators' });
  },
});
