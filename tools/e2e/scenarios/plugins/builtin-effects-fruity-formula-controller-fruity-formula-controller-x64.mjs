// plugins/builtin-effects-fruity-formula-controller-fruity-formula-controller-x64: Fruity Formula Controller (effects, DLL Fruity Formula Controller_x64.dll (3869696 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-formula-controller-fruity-formula-controller-x64',
  covers: ['plugins/effects/fruity-formula-controller-fruity-formula-controller-x64', 'plugins/window/effects-fruity-formula-controller-fruity-formula-controller-x64', 'plugins/param/effects-fruity-formula-controller-fruity-formula-controller-x64', 'plugins/audio/effects-fruity-formula-controller-fruity-formula-controller-x64', 'plugins/stop/effects-fruity-formula-controller-fruity-formula-controller-x64'],
  expectFail: '#1442',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Formula Controller", kind: 'effects' });
  },
});
