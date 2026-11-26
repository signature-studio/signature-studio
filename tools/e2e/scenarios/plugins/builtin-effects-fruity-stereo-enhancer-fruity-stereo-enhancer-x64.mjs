// plugins/builtin-effects-fruity-stereo-enhancer-fruity-stereo-enhancer-x64: Fruity Stereo Enhancer (effects, DLL Fruity Stereo Enhancer_x64.dll (3728384 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-stereo-enhancer-fruity-stereo-enhancer-x64',
  covers: ['plugins/effects/fruity-stereo-enhancer-fruity-stereo-enhancer-x64', 'plugins/window/effects-fruity-stereo-enhancer-fruity-stereo-enhancer-x64', 'plugins/param/effects-fruity-stereo-enhancer-fruity-stereo-enhancer-x64', 'plugins/audio/effects-fruity-stereo-enhancer-fruity-stereo-enhancer-x64', 'plugins/stop/effects-fruity-stereo-enhancer-fruity-stereo-enhancer-x64'],
  expectFail: '#1449',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Stereo Enhancer", kind: 'effects' });
  },
});
