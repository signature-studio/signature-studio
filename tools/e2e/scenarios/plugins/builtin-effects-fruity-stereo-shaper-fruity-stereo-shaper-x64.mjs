// plugins/builtin-effects-fruity-stereo-shaper-fruity-stereo-shaper-x64: Fruity Stereo Shaper (effects, DLL Fruity Stereo Shaper_x64.dll (3800064 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-stereo-shaper-fruity-stereo-shaper-x64',
  covers: ['plugins/effects/fruity-stereo-shaper-fruity-stereo-shaper-x64', 'plugins/window/effects-fruity-stereo-shaper-fruity-stereo-shaper-x64', 'plugins/param/effects-fruity-stereo-shaper-fruity-stereo-shaper-x64', 'plugins/audio/effects-fruity-stereo-shaper-fruity-stereo-shaper-x64', 'plugins/stop/effects-fruity-stereo-shaper-fruity-stereo-shaper-x64'],
  expectFail: '#1450',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Stereo Shaper", kind: 'effects' });
  },
});
