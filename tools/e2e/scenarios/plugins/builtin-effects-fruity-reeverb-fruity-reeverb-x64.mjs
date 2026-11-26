// plugins/builtin-effects-fruity-reeverb-fruity-reeverb-x64: Fruity Reeverb (effects, DLL Fruity Reeverb_x64.dll (3714048 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-reeverb-fruity-reeverb-x64',
  covers: ['plugins/effects/fruity-reeverb-fruity-reeverb-x64', 'plugins/window/effects-fruity-reeverb-fruity-reeverb-x64', 'plugins/param/effects-fruity-reeverb-fruity-reeverb-x64', 'plugins/audio/effects-fruity-reeverb-fruity-reeverb-x64', 'plugins/stop/effects-fruity-reeverb-fruity-reeverb-x64'],
  expectFail: '#1447',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Reeverb", kind: 'effects' });
  },
});
