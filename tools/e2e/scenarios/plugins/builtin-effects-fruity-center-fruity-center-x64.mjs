// plugins/builtin-effects-fruity-center-fruity-center-x64: Fruity Center (effects, DLL Fruity Center_x64.dll (3588608 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-center-fruity-center-x64',
  covers: ['plugins/effects/fruity-center-fruity-center-x64', 'plugins/window/effects-fruity-center-fruity-center-x64', 'plugins/param/effects-fruity-center-fruity-center-x64', 'plugins/audio/effects-fruity-center-fruity-center-x64', 'plugins/stop/effects-fruity-center-fruity-center-x64'],
  expectFail: '#1438',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Center", kind: 'effects' });
  },
});
