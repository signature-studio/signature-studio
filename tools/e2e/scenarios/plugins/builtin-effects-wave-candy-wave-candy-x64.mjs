// plugins/builtin-effects-wave-candy-wave-candy-x64: Wave Candy (effects, DLL Wave Candy_x64.dll (4348928 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-wave-candy-wave-candy-x64',
  covers: ['plugins/effects/wave-candy-wave-candy-x64', 'plugins/window/effects-wave-candy-wave-candy-x64', 'plugins/param/effects-wave-candy-wave-candy-x64', 'plugins/audio/effects-wave-candy-wave-candy-x64', 'plugins/stop/effects-wave-candy-wave-candy-x64'],
  expectFail: '#1462',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Wave Candy", kind: 'effects' });
  },
});
