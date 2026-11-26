// plugins/builtin-effects-fruity-reeverb-2-fruity-reeverb-2-x64: Fruity Reeverb 2 (effects, DLL Fruity Reeverb 2_x64.dll (3831808 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-reeverb-2-fruity-reeverb-2-x64',
  covers: ['plugins/effects/fruity-reeverb-2-fruity-reeverb-2-x64', 'plugins/window/effects-fruity-reeverb-2-fruity-reeverb-2-x64', 'plugins/param/effects-fruity-reeverb-2-fruity-reeverb-2-x64', 'plugins/audio/effects-fruity-reeverb-2-fruity-reeverb-2-x64', 'plugins/stop/effects-fruity-reeverb-2-fruity-reeverb-2-x64'],
  expectFail: '#1447',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Reeverb 2", kind: 'effects' });
  },
});
