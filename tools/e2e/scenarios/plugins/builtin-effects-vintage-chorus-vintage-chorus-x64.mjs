// plugins/builtin-effects-vintage-chorus-vintage-chorus-x64: Vintage Chorus (effects, DLL Vintage Chorus_x64.dll (3941888 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-vintage-chorus-vintage-chorus-x64',
  covers: ['plugins/effects/vintage-chorus-vintage-chorus-x64', 'plugins/window/effects-vintage-chorus-vintage-chorus-x64', 'plugins/param/effects-vintage-chorus-vintage-chorus-x64', 'plugins/audio/effects-vintage-chorus-vintage-chorus-x64', 'plugins/stop/effects-vintage-chorus-vintage-chorus-x64'],
  expectFail: '#1461',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Vintage Chorus", kind: 'effects' });
  },
});
