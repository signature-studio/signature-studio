// plugins/builtin-effects-fruity-panomatic-fruity-panomatic-x64: Fruity PanOMatic (effects, DLL Fruity PanOMatic_x64.dll (3759616 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-panomatic-fruity-panomatic-x64',
  covers: ['plugins/effects/fruity-panomatic-fruity-panomatic-x64', 'plugins/window/effects-fruity-panomatic-fruity-panomatic-x64', 'plugins/param/effects-fruity-panomatic-fruity-panomatic-x64', 'plugins/audio/effects-fruity-panomatic-fruity-panomatic-x64', 'plugins/stop/effects-fruity-panomatic-fruity-panomatic-x64'],
  expectFail: '#1445',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity PanOMatic", kind: 'effects' });
  },
});
