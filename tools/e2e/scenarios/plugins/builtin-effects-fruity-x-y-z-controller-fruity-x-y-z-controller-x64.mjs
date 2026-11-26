// plugins/builtin-effects-fruity-x-y-z-controller-fruity-x-y-z-controller-x64: Fruity X-Y-Z Controller (effects, DLL Fruity X-Y-Z Controller_x64.dll (5796864 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-x-y-z-controller-fruity-x-y-z-controller-x64',
  covers: ['plugins/effects/fruity-x-y-z-controller-fruity-x-y-z-controller-x64', 'plugins/window/effects-fruity-x-y-z-controller-fruity-x-y-z-controller-x64', 'plugins/param/effects-fruity-x-y-z-controller-fruity-x-y-z-controller-x64', 'plugins/audio/effects-fruity-x-y-z-controller-fruity-x-y-z-controller-x64', 'plugins/stop/effects-fruity-x-y-z-controller-fruity-x-y-z-controller-x64'],
  expectFail: '#1451',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity X-Y-Z Controller", kind: 'effects' });
  },
});
