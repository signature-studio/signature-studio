// plugins/builtin-effects-hyper-chorus-hyper-chorus-x64: Hyper Chorus (effects, DLL Hyper Chorus_x64.dll (3938304 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-hyper-chorus-hyper-chorus-x64',
  covers: ['plugins/effects/hyper-chorus-hyper-chorus-x64', 'plugins/window/effects-hyper-chorus-hyper-chorus-x64', 'plugins/param/effects-hyper-chorus-hyper-chorus-x64', 'plugins/audio/effects-hyper-chorus-hyper-chorus-x64', 'plugins/stop/effects-hyper-chorus-hyper-chorus-x64'],
  expectFail: '#1453',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Hyper Chorus", kind: 'effects' });
  },
});
