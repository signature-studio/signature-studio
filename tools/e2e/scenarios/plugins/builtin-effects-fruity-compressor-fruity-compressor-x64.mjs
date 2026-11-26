// plugins/builtin-effects-fruity-compressor-fruity-compressor-x64: Fruity Compressor (effects, DLL Fruity Compressor_x64.dll (3722752 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-compressor-fruity-compressor-x64',
  covers: ['plugins/effects/fruity-compressor-fruity-compressor-x64', 'plugins/window/effects-fruity-compressor-fruity-compressor-x64', 'plugins/param/effects-fruity-compressor-fruity-compressor-x64', 'plugins/audio/effects-fruity-compressor-fruity-compressor-x64', 'plugins/stop/effects-fruity-compressor-fruity-compressor-x64'],
  expectFail: '#1439',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Compressor", kind: 'effects' });
  },
});
