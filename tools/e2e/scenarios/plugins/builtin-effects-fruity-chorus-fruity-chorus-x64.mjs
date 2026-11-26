// plugins/builtin-effects-fruity-chorus-fruity-chorus-x64: Fruity Chorus (effects, DLL Fruity Chorus_x64.dll (3744256 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-chorus-fruity-chorus-x64',
  covers: ['plugins/effects/fruity-chorus-fruity-chorus-x64', 'plugins/window/effects-fruity-chorus-fruity-chorus-x64', 'plugins/param/effects-fruity-chorus-fruity-chorus-x64', 'plugins/audio/effects-fruity-chorus-fruity-chorus-x64', 'plugins/stop/effects-fruity-chorus-fruity-chorus-x64'],
  expectFail: '#1439',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Chorus", kind: 'effects' });
  },
});
